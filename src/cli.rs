use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::sync;

/// Sync selected files and directories between machines over SSH.
#[derive(Parser)]
#[command(version)]
pub struct Args {
    /// Options controlling how transfers are performed.
    #[command(flatten)]
    pub opts: sync::Options,

    /// Command to run.
    #[command(subcommand)]
    pub cmd: Command,
}

/// Available commands.
#[derive(Subcommand)]
pub enum Command {
    /// Push tracked paths to all configured targets.
    Push,

    /// Inspect or initialize the local configuration.
    Config {
        /// Configuration action.
        #[command(subcommand)]
        cmd: ConfigCmd,
    },

    /// Remove tracked paths that no longer exist locally from the config.
    Prune,

    /// Pull tracked paths from a remote machine.
    Pull {
        /// Machine to pull from.
        machine: String,
    },

    /// Start tracking one or more paths under the home directory.
    Track {
        /// Paths to add to the config.
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<PathBuf>,
    },

    /// Stop tracking one or more paths already present in the config.
    Untrack {
        /// Paths to remove from the config.
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<PathBuf>,
    },

    /// Manage configured machines.
    Machine {
        /// Machine action.
        #[command(subcommand)]
        cmd: MachineCmd,
    },
}

/// Machine management commands.
#[derive(Subcommand)]
pub enum MachineCmd {
    /// Add one or more machine names to the config.
    Add {
        /// Names to add.
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },

    /// Remove one or more machine names from the config.
    #[command(alias = "rm")]
    Remove {
        /// Names to remove.
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },

    /// List configured machines.
    #[command(alias = "ls")]
    List,
}

/// Configuration commands.
#[derive(Subcommand)]
pub enum ConfigCmd {
    /// Create a new config for the given main machine.
    Init {
        /// Name of the main machine.
        main_machine: String,
    },

    /// Print the current config as JSON.
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
