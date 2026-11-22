use bluer::{
    Address, InvalidAddress,
    rfcomm::{SocketAddr, Stream},
};

use practica_5::{
    app::*,
    bluetooth::{Adaptador, BluetoothConnection},
    cli::{Command, Param},
    tui::Tui,
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

    #[error(transparent)]
    App(#[from] AppError),
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
            let (tx, rx, task) = App::initialize_bluetooth(BluetoothConnection::new(
                Stream::connect(SocketAddr::new(address, 1)).await?,
            ));
            let mut app = App::new(tx, rx, task);
            let mut tui = Tui::new()?;
            tui.init()?;
            loop {
                app.receive_status()?;
                tui.draw(app.state())?;
            }
        }
    }
    Ok(())
}
