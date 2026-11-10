use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Push,
    Pull { machine: String },
}
