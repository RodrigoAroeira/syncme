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
    Pull {
        machine: String,
    },
    Track {
        path: PathBuf,
    },
    Untrack {
        path: PathBuf,
    },
    InitConfig {
        main_machine: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }
}
