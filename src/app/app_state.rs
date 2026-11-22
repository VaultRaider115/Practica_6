use super::AppError;
use crate::motor::{ConsumptionAccumulator, CurrentMa, calc};
use crate::{
    bluetooth::{BluetoothConnection, MotorCommand, MotorControl, MotorMode, MotorStatus},
    motor::{MotorState, ResponseStatus},
};
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender, channel, error::TryRecvError};
use tokio::task::JoinHandle;

pub struct App {
    command_tx: Sender<u8>,
    status_rx: Receiver<Vec<u8>>,
    tasks: Task,
    state: MotorState,
    consumption: ConsumptionAccumulator,
}

pub struct Task {
    pub tasks: Vec<JoinHandle<Result<(), AppError>>>,
}

impl App {
    pub fn new(sender: Sender<u8>, receiver: Receiver<Vec<u8>>, task: Task) -> Self {
        Self {
            command_tx: sender,
            status_rx: receiver,
            tasks: task,
            state: MotorState::new(),
            consumption: ConsumptionAccumulator::new(),
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
            loop {
                let data = reader.receive_message().await?;
                status_tx.send(data).await?
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
        match self.status_rx.try_recv() {
            Ok(data) => {
                let message = String::from_utf8(data)?;
                let parser = ResponseStatus::parse_response(&message)?;
                self.state.set_current_step(parser.current_step)?;
                self.state.set_coils(parser.coils);
                Ok(())
            }
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => Err(AppError::Recv(TryRecvError::Disconnected)),
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
        Ok(calc(
            self.state.mode()?,
            self.state.status()?,
            self.state.current_step()?,
        ))
    }

    pub fn update_consumption(&mut self, elapsed: Duration) -> Result<(), AppError> {
        self.consumption.accumulate(self.current()?, elapsed)?;
        Ok(())
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
        assert!(app.state().status().is_err());
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
}
