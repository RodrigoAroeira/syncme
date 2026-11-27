use anyhow::{Context, Result};
use clap::Parser;

mod cli;
mod commands;
mod config;
mod sync;
mod util;

use config::Config;

fn main() -> Result<()> {
    use cli::Command as Cmd;
    use cli::ConfigCmd as CCmd;

    let home = dirs::home_dir().context("Unable to get home dir")?;
    let cli::Args { opts, cmd } = cli::Args::parse();

    // Handling this case earlier allows for a single creation point for conf
    // instead of creating inside of every block in the match
    if let Cmd::Config {
        cmd: CCmd::Init { main_machine },
    } = &cmd
    {
        return commands::init(main_machine);
    }

    let mut conf = Config::load()?;
    match cmd {
        Cmd::Push => commands::push(&conf, &home, &opts)?,
        Cmd::Config { cmd } => match cmd {
            CCmd::Init { .. } => unreachable!("Handled earlier"),
            CCmd::Print => commands::print(&conf)?,
        },
        Cmd::Prune => commands::prune(&mut conf, opts.verbose)?,
        Cmd::Pull { machine } => commands::pull(&conf, &machine, &home, &opts)?,
        Cmd::Track { paths } => commands::track(&mut conf, &paths, &home)?,
        Cmd::Untrack { paths } => commands::untrack(&mut conf, &paths)?,
        Cmd::Machine { cmd } => commands::machine(&mut conf, cmd)?,
    }
    Ok(())
}
