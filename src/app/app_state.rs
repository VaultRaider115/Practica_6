use super::AppError;
use crate::motor::{ArduinoMessage, ConsumptionAccumulator, CurrentMa, calc};
use crate::{
    bluetooth::{BluetoothConnection, MotorCommand, MotorControl, MotorMode, MotorStatus},
    motor::MotorState,
};
use std::{collections::VecDeque, time::Duration};
use tokio::sync::mpsc::{Receiver, Sender, channel, error::TryRecvError};
use tokio::task::JoinHandle;

pub struct App {
    command_tx: Sender<u8>,
    status_rx: Receiver<Vec<u8>>,
    tasks: Task,
    state: MotorState,
    consumption: ConsumptionAccumulator,
    current_history: VecDeque<CurrentMa>,
    angle: AngleTracker,
}

pub struct Task {
    pub tasks: Vec<JoinHandle<Result<(), AppError>>>,
}

/// Saca de `buf` los mensajes completos del Arduino ("P3 0110", "DONE", "STOPPED").
/// Se reconocen por su forma, no por el salto de línea.
fn take_messages(buf: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let skip = buf.iter().take_while(|b| b.is_ascii_whitespace()).count();
        buf.drain(..skip);
        let len = match buf.first() {
            None => break,
            Some(b'P') | Some(b'S') => 7, // "P3 0110" / "STOPPED"
            Some(b'D') => 4,              // "DONE"
            Some(_) => {
                buf.remove(0); // basura: descarta y resincroniza
                continue;
            }
        };
        if buf.len() < len {
            break;
        }
        out.push(buf.drain(..len).collect());
    }
    out
}

impl App {
    pub fn new(sender: Sender<u8>, receiver: Receiver<Vec<u8>>, task: Task) -> Self {
        Self {
            command_tx: sender,
            status_rx: receiver,
            tasks: task,
            state: MotorState::new(),
            consumption: ConsumptionAccumulator::new(),
            current_history: VecDeque::with_capacity(15),
            angle: AngleTracker::default(),
        }
    }

    pub fn initialize_bluetooth(
        connecion: BluetoothConnection,
    ) -> (Sender<u8>, Receiver<Vec<u8>>, Task) {
        let (mut reader, mut writer) = connecion.into_parts();

        let (command_tx, mut command_rx) = channel::<u8>(32);
        let (status_tx, status_rx) = channel::<Vec<u8>>(32);

        let task_1 = tokio::spawn(async move {
            while let Some(comando) = command_rx.recv().await {
                writer.send(comando).await?;
            }
            Ok::<(), AppError>(())
        });

        let task_2 = tokio::spawn(async move {
            let mut buf = Vec::new();
            loop {
                buf.extend(reader.receive_message().await?);
                for msg in take_messages(&mut buf) {
                    status_tx.send(msg).await?;
                }
            }
        });

        let task = Task {
            tasks: vec![task_1, task_2],
        };

        (command_tx, status_rx, task)
    }

    pub fn state(&self) -> &MotorState {
        &self.state
    }

    pub async fn send_command(&mut self, command: MotorCommand) -> Result<(), AppError> {
        let command_byte: u8 = command.into();
        match self.command_tx.send(command_byte).await {
            Ok(_) => {
                self.state.set_mode(MotorMode::from(command));
                self.state.set_status(MotorStatus::Active);
                Ok(())
            }
            Err(error) => Err(AppError::from(error)),
        }
    }

    pub async fn stop_motor(&mut self, command: MotorControl) -> Result<(), AppError> {
        let command_byte: u8 = command.into();
        match self.command_tx.send(command_byte).await {
            Ok(_) => {
                self.state.clear_mode();
                self.state.set_status(MotorStatus::Stopped);
                Ok(())
            }
            Err(error) => Err(AppError::from(error)),
        }
    }

    pub fn receive_status(&mut self) -> Result<(), AppError> {
        loop {
            match self.status_rx.try_recv() {
                Ok(data) => {
                    let message = String::from_utf8(data)?;
                    match ArduinoMessage::parse(&message)? {
                        ArduinoMessage::Step(status) => {
                            self.state.set_current_step(status.current_step)?;
                            self.angle.update(status.current_step);
                            self.state.set_coils(status.coils);
                        }
                        ArduinoMessage::Done | ArduinoMessage::Stopped => {
                            self.state.set_coils([false; 4]);
                            self.state.clear_mode();
                            self.state.set_status(MotorStatus::Stopped);
                        }
                    }
                }
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err(AppError::Recv(TryRecvError::Disconnected));
                }
            }
        }
    }

    pub async fn shutdown(self) -> Result<(), AppError> {
        for task in &self.tasks.tasks {
            task.abort();
        }

        for task in self.tasks.tasks {
            match task.await {
                Ok(Ok(_)) => continue,
                Ok(Err(error)) => return Err(error),
                Err(error) => {
                    if !error.is_cancelled() {
                        return Err(AppError::from(error));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn accumulator(&self) -> &ConsumptionAccumulator {
        &self.consumption
    }

    pub fn current(&self) -> Result<CurrentMa, AppError> {
        match self.state.status()? {
            MotorStatus::Active => Ok(calc(
                self.state.mode()?,
                self.state.status()?,
                self.state.current_step()?,
            )),
            MotorStatus::Stopped => Ok(0),
        }
    }

    pub fn update_consumption(&mut self, elapsed: Duration) -> Result<(), AppError> {
        let data = self.current()?;
        self.consumption.accumulate(data, elapsed)?;

        if self.current_history.len() >= 15 {
            self.current_history.pop_front();
        }

        self.current_history.push_back(data);
        Ok(())
    }

    pub fn current_history(&self) -> &VecDeque<CurrentMa> {
        &self.current_history
    }

    pub fn is_initialized(&self) -> bool {
        self.state.is_initialized()
    }

    pub fn angle_degrees(&self) -> f64 {
        self.angle.degrees()
    }
}

#[derive(Debug, Default)]
pub struct AngleTracker {
    last_step: Option<usize>,
    half_steps: i64,
}

impl AngleTracker {
    pub fn update(&mut self, step: usize) {
        if let Some(last) = self.last_step {
            let d = (step + 8 - last) % 8;
            self.half_steps += if d <= 4 { d as i64 } else { d as i64 - 8 };
        }
        self.last_step = Some(step);
    }

    pub fn degrees(&self) -> f64 {
        self.half_steps as f64 * 360.0 / 4096.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn create_app() -> (App, Receiver<u8>, Sender<Vec<u8>>) {
        let (command_tx, command_rx) = mpsc::channel::<u8>(32);
        let (status_tx, status_rx) = mpsc::channel::<Vec<u8>>(32);

        let task = Task { tasks: Vec::new() };

        let app = App::new(command_tx, status_rx, task);

        (app, command_rx, status_tx)
    }

    #[tokio::test]
    async fn test_send_command() {
        let (mut app, mut command_rx, _) = create_app();

        app.send_command(MotorCommand::SimpleLeft).await.unwrap();

        assert_eq!(command_rx.recv().await, Some(b'a'));
        assert_eq!(format!("{:?}", app.state().mode().unwrap()), "Simple");
        assert_eq!(format!("{:?}", app.state().status().unwrap()), "Active");
    }

    #[tokio::test]
    async fn test_stop_motor() {
        let (mut app, mut command_rx, _) = create_app();

        app.send_command(MotorCommand::SimpleLeft).await.unwrap();

        command_rx.recv().await;

        app.stop_motor(MotorControl::Stop).await.unwrap();

        assert_eq!(command_rx.recv().await, Some(b'x'));
        assert!(app.state().mode().is_err());

        assert_eq!(format!("{:?}", app.state().status().unwrap()), "Stopped");
    }

    #[tokio::test]
    async fn test_send_command_disconnected() {
        let (mut app, command_rx, _) = create_app();

        drop(command_rx);

        assert!(app.send_command(MotorCommand::SimpleLeft).await.is_err());

        assert!(app.state().mode().is_err());
        assert!(matches!(app.state().status(), Ok(MotorStatus::Stopped))); // el envío falló: sigue en el valor inicial
    }

    #[tokio::test]
    async fn test_receive_status() {
        let (mut app, _, status_tx) = create_app();

        status_tx.send(b"P3 0110".to_vec()).await.unwrap();

        app.receive_status().unwrap();

        assert_eq!(format!("{:?}", app.state().current_step().unwrap()), "3");
        assert_eq!(
            format!("{:?}", app.state().coils().unwrap()),
            "[false, true, true, false]"
        );
    }

    #[tokio::test]
    async fn test_receive_status_invalid_message() {
        let (mut app, _, status_tx) = create_app();

        status_tx.send(b"INVALID".to_vec()).await.unwrap();

        assert!(app.receive_status().is_err());
    }

    #[tokio::test]
    async fn test_receive_status_empty() {
        let (mut app, _, _status_tx) = create_app();

        assert!(app.receive_status().is_ok());
    }

    #[tokio::test]
    async fn test_receive_status_disconnected() {
        let (mut app, _, status_tx) = create_app();

        drop(status_tx);

        assert!(app.receive_status().is_err());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let (command_tx, status_rx) = {
            let (tx, _) = mpsc::channel::<u8>(32);
            let (_, rx) = mpsc::channel::<Vec<u8>>(32);
            (tx, rx)
        };

        let task = tokio::spawn(async { std::future::pending::<Result<(), AppError>>().await });

        let app = App::new(command_tx, status_rx, Task { tasks: vec![task] });

        assert!(app.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_receive_done_stops_motor() {
        let (mut app, mut command_rx, status_tx) = create_app();
        app.send_command(MotorCommand::SimpleLeft).await.unwrap();
        command_rx.recv().await;

        status_tx.send(b"DONE\r\n".to_vec()).await.unwrap();
        app.receive_status().unwrap();

        assert_eq!(format!("{:?}", app.state().status().unwrap()), "Stopped");
        assert!(app.state().mode().is_err());
    }

    #[tokio::test]
    async fn test_receive_status_drains_queue() {
        let (mut app, _, status_tx) = create_app();
        status_tx.send(b"P1 1100".to_vec()).await.unwrap();
        status_tx.send(b"P2 0110".to_vec()).await.unwrap();

        app.receive_status().unwrap();

        assert_eq!(format!("{:?}", app.state().current_step().unwrap()), "2");
    }

    #[test]
    fn take_messages_split_reads_and_no_newlines() {
        let mut buf = b"P3 01".to_vec();
        assert!(take_messages(&mut buf).is_empty());

        buf.extend(b"10\r\nP4 1100\r\nDO");
        assert_eq!(
            take_messages(&mut buf),
            vec![b"P3 0110".to_vec(), b"P4 1100".to_vec()]
        );

        buf.extend(b"NE");
        assert_eq!(take_messages(&mut buf), vec![b"DONE".to_vec()]);

        let mut buf = b"3 0110\r\nP4 1100".to_vec(); // arranca a mitad de mensaje
        assert_eq!(take_messages(&mut buf), vec![b"P4 1100".to_vec()]);
    }

    #[test]
    fn angle_quarter_turn_right_simple_mode() {
        let mut a = AngleTracker::default();
        a.update(0); // la primera línea solo fija la referencia
        for i in 1..=512 {
            a.update((i * 2) % 8);
        }
        assert_eq!(a.degrees(), 90.0);
    }

    #[test]
    fn angle_left_goes_negative() {
        let mut a = AngleTracker::default();
        a.update(0);
        a.update(7);
        a.update(6);
        assert_eq!(a.degrees(), -2.0 * 360.0 / 4096.0);
    }

    #[test]
    fn angle_alignment_jump_counts() {
        let mut a = AngleTracker::default();
        a.update(1);
        a.update(4); // alinea (+1) y avanza 2
        assert_eq!(a.degrees(), 3.0 * 360.0 / 4096.0);

        let mut b = AngleTracker::default();
        b.update(1);
        b.update(0); // alinea (+1) y retrocede 2
        assert_eq!(b.degrees(), -360.0 / 4096.0);
    }

    #[test]
    fn angle_full_turn_wraps() {
        let mut a = AngleTracker::default();
        a.update(0);
        for i in 1..=4096 {
            a.update(i % 8);
        }
        assert_eq!(a.degrees(), 360.0);
    }
}
