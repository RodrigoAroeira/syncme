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

    /// Pull tracked paths from the hub, or from a peer when no hub is set.
    Pull {
        /// Peer to pull from when no hub is configured.
        peer: Option<String>,
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

    /// Manage configured peers.
    Peer {
        /// Peer action.
        #[command(subcommand)]
        cmd: PeerCmd,
    },
}

/// Peer management commands.
#[derive(Subcommand)]
pub enum PeerCmd {
    /// Add one or more peer names to the config.
    Add {
        /// Names to add.
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },

    /// Remove one or more peer names from the config.
    #[command(alias = "rm")]
    Remove {
        /// Names to remove.
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },

    /// List configured peers.
    #[command(alias = "ls")]
    List,
}

/// Configuration commands.
#[derive(Subcommand)]
pub enum ConfigCmd {
    /// Create a peer config, a peer-with-hub config, or a hub config.
    Init {
        /// Hub SSH name or address; omit for peer-to-peer mode.
        hub: Option<String>,

        /// Create a hub config instead of a peer config.
        #[arg(long = "hub", conflicts_with = "hub")]
        as_hub: bool,
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
