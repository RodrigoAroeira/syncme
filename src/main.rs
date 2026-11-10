#![expect(dead_code, unused_variables)]

use anyhow::Result;
use clap::Parser;

mod cli;
mod config;

fn main() -> Result<()> {
    use cli::Command as Cmd;
    let args = cli::Args::parse();
    let mut conf = config::Config::load()?;
    match args.cmd {
        Cmd::Push => todo!(),
        Cmd::Pull { machine } => todo!(),
        Cmd::Sync { path } => {
            conf.add_path(path);
            conf.save()?;
        }
    }
    Ok(())
}
