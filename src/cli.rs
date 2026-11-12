use std::path::PathBuf;

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
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    Prune {
        #[arg(short, long)]
        verbose: bool,
    },
    Pull {
        machine: String,
    },
    Track {
        path: PathBuf,
    },
    Untrack {
        path: PathBuf,
    },
    Machine {
        #[command(subcommand)]
        cmd: MachineCmd,
    },
}

#[derive(Subcommand)]
pub enum MachineCmd {
    Add {
        name: String,
    },
    #[command(alias = "rm")]
    Remove {
        name: String,
    },
    #[command(alias = "ls")]
    List,
}

#[derive(Subcommand)]
pub enum ConfigCmd {
    Init { main_machine: String },
    Print,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }
}
