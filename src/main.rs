#![expect(dead_code, unused_variables)]

use anyhow::{Result, ensure};
use clap::Parser;

mod cli;
mod config;

fn main() -> Result<()> {
    use cli::Command as Cmd;
    let args = cli::Args::parse();
    match args.cmd {
        Cmd::Push => todo!(),
        Cmd::Pull { machine } => todo!(),
        Cmd::Sync { path } => {
            let mut conf = config::Config::load()?;
            conf.add_path(path);
            conf.save()?;
        }
        Cmd::InitConfig { main_machine } => {
            ensure!(config::Config::load().is_err(), "config already exists");
            config::Config::new(main_machine).save()?;
        }
    }
    Ok(())
}
