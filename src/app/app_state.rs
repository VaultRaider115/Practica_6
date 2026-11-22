use tokio::sync::mpsc::{Receiver, Sender, channel, error::TryRecvError};
use tokio::task::JoinHandle;

use super::AppError;

use crate::{
    bluetooth::{BluetoothConnection, MotorCommand, MotorControl, MotorMode, MotorStatus},
    motor::{MotorState, ResponseStatus},
};

pub struct App {
    command_tx: Sender<u8>,
    status_rx: Receiver<Vec<u8>>,
    tasks: Task,
    state: MotorState,
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
}
