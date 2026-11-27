use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::sync;

#[derive(Parser)]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub opts: sync::Options,

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
    Prune,
    Pull {
        machine: String,
    },
    Track {
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<PathBuf>,
    },
    Untrack {
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<PathBuf>,
    },
    Machine {
        #[command(subcommand)]
        cmd: MachineCmd,
    },
}

#[derive(Subcommand)]
pub enum MachineCmd {
    Add {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },
    #[command(alias = "rm")]
    Remove {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
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
