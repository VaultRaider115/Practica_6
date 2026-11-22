use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Command {
    Scan,
    Connect { mac: String },
}

#[derive(Parser)]
pub struct Param {
    #[command(subcommand)]
    pub command: Command,
}
