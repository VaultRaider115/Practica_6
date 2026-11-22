mod app;
mod bluetooth;
mod cli;
mod motor;

use crate::bluetooth::{Adaptador, BluetoothConnection};
use crate::cli::{Command, Param};
use bluer::{
    Address, InvalidAddress,
    rfcomm::{SocketAddr, Stream},
};
use clap::Parser;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error en el driver")]
    Bluer(#[from] bluer::Error),

    #[error("Error en el parser")]
    Parser(#[from] InvalidAddress),

    #[error("Error de entrada/salida")]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let params = Param::parse();
    match params.command {
        Command::Scan => {
            let adaptador = Adaptador::new().await?;
            adaptador.start_discover().await?;
        }
        Command::Connect { mac } => {
            let address = Address::from_str(&mac)?;
            let connectio = Stream::connect(SocketAddr::new(address, 1)).await?;
        }
    }
    Ok(())
}
