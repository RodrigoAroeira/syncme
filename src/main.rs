#![expect(dead_code, unused_variables)]

use anyhow::{Context, Result, ensure};
use clap::Parser;
use path_absolutize::Absolutize;

mod cli;
mod config;
mod sync;

use config::Config;

fn main() -> Result<()> {
    use cli::Command as Cmd;
    let home = dirs::home_dir().context("Unable to get home dir")?;
    let args = cli::Args::parse();
    match args.cmd {
        Cmd::Push => {
            let conf = Config::load()?;
            for file in conf.paths() {
                let path_str = file.to_str().context("invalid UTF-8 path")?;

                let remote_rel = file.strip_prefix(&home).unwrap_or(file);
                let remote_path = remote_rel.to_str().context("invalid UTF-8 path")?;
                if conf.this() == conf.main() {
                    // Send to other machines
                    for machine in conf.machines() {
                        sync::rsync(path_str, &format!("{machine}:{remote_path}"))?;
                    }
                } else {
                    // Send to main machine
                    sync::rsync(path_str, &format!("{}:{remote_path}", conf.main()))?;
                }
            }
        }
        Cmd::Pull { machine } => todo!(),
        Cmd::Sync { path } => {
            let path = path.absolutize()?;
            let mut conf = Config::load()?;
            conf.add_path(path);
            conf.save()?;
        }
        Cmd::InitConfig { main_machine } => {
            ensure!(Config::load().is_err(), "config already exists");
            Config::new(main_machine).save()?;
        }
    }
    Ok(())
}
