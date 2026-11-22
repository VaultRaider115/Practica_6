use crate::motor::{ConsumptionError, MotorError, ParserError};
use std::{io, string::FromUtf8Error};
use tokio::{
    sync::mpsc::error::{SendError, TryRecvError},
    task::JoinError,
};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Bluetooth(#[from] bluer::Error),

    #[error(transparent)]
    Motor(#[from] MotorError),

    #[error(transparent)]
    Consumption(#[from] ConsumptionError),

    #[error(transparent)]
    ParserError(#[from] ParserError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Utf8(#[from] FromUtf8Error),

    #[error(transparent)]
    SendVec(#[from] SendError<Vec<u8>>),

    #[error(transparent)]
    Send(#[from] SendError<u8>),

    #[error(transparent)]
    Recv(#[from] TryRecvError),

    #[error("Error en una tarea de Tokio: {0}")]
    Task(#[from] JoinError),
}
