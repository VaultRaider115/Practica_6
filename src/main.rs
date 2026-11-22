use bluer::{
    Address, InvalidAddress,
    rfcomm::{SocketAddr, Stream},
};

use crossterm::event::KeyCode;
use practica_5::{
    app::*,
    bluetooth::{Adaptador, BluetoothConnection, MotorCommand, MotorControl},
    cli::{Command, Param},
    tui::{Tui, events::InputHandler},
};

use clap::Parser;
use std::{
    str::FromStr,
    time::{Duration, Instant},
};

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

pub enum Action {
    Command(MotorCommand),
    Control(MotorControl),
    Quit,
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
            let mut last_update = Instant::now();
            let mut timer = tokio::time::interval(Duration::from_millis(100));
            let mut input = InputHandler::default();
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let elapsed = last_update.elapsed();
                        app.receive_status()?;
                        app.update_consumption(elapsed)?;
                        tui.draw(app.state())?;
                        last_update = Instant::now();
                        Ok(())
                    }

                    event = input.next_event() => {
                        match event {
                            Ok(Some(key)) => {
                                let action = match key.code {
                                    KeyCode::Char('a') => Some(Action::Command(MotorCommand::SimpleLeft)),
                                    KeyCode::Char('b') => Some(Action::Command(MotorCommand::SimpleRight)),
                                    KeyCode::Char('c') => Some(Action::Command(MotorCommand::HalfStepLeft)),
                                    KeyCode::Char('d') => Some(Action::Command(MotorCommand::HalfStepRight)),
                                    KeyCode::Char('e') => Some(Action::Command(MotorCommand::DoubleStepLeft)),
                                    KeyCode::Char('f') => Some(Action::Command(MotorCommand::DoubleStepRight)),
                                    KeyCode::Char('x') => Some(Action::Control(MotorControl::Stop)),
                                    KeyCode::Char('q') => Some(Action::Quit),
                                    _ => None,
                                };

                                match action {
                                    Some(Action::Command(command)) => app.send_command(command).await?,
                                    Some(Action::Control(command)) => app.stop_motor(command).await?,
                                    Some(Action::Quit) => break,
                                    None => {}
                                }
                                Ok(())
                            }
                            Ok(None) => Ok(()),
                            Err(error) => Err(Error::from(error)),
                        }
                    }
                }?;
            }
            app.shutdown().await?;
            tui.restore()?;
        }
    }
    Ok(())
}
