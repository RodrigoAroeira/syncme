use anyhow::{Context, Result, ensure};
use clap::Parser;
use path_absolutize::Absolutize;

mod cli;
mod config;
mod sync;

use config::Config;

fn main() -> Result<()> {
    use cli::Command as Cmd;
    use cli::MachineCmd as MCmd;

    let home = dirs::home_dir().context("Unable to get home dir")?;
    let args = cli::Args::parse();
    match args.cmd {
        Cmd::Push => {
            let conf = Config::load()?;
            for file in conf.paths() {
                let path_str = file.to_str().context("invalid UTF-8 path")?;

                let remote_rel = sync::remote_path(file, &home);
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
        Cmd::Pull { machine } => {
            let conf = Config::load()?;
            // Not sure if this ensure is correct
            ensure!(
                conf.this() != conf.main(),
                "Nothing to do: already on main machine",
            );

            ensure!(
                conf.this() != machine,
                "Nothing to do: target is current machine",
            );
            ensure!(
                conf.machines().contains(&machine),
                "target machine not found in configured machines",
            );

            for file in conf.paths() {
                let local_path = file.to_str().context("invalid UTF-8 path")?;

                let remote_path = sync::remote_path(file, &home)
                    .to_str()
                    .map(String::from)
                    .context("invalid UTF-8 path")?;

                sync::rsync(&format!("{machine}:{remote_path}"), local_path)?;
            }
        }
        Cmd::Track { path } => {
            let path = path.absolutize()?;
            let mut conf = Config::load()?;
            conf.add_path(path);
            conf.save()?;
        }
        Cmd::Untrack { path } => {
            let path = path.absolutize()?;
            let mut conf = Config::load()?;
            conf.untrack_path(path)?;
            conf.save()?;
        }
        Cmd::InitConfig { main_machine } => {
            ensure!(Config::load().is_err(), "config already exists");
            Config::new(main_machine).save()?;
        }
        Cmd::Machine { cmd } => {
            let mut conf = Config::load()?;
            match cmd {
                MCmd::Add { name } => conf.add_machine(name),
                MCmd::Remove { name } => conf.rm_machine(&name)?,
                MCmd::List => {
                    for m in conf.machines() {
                        println!("- {m}");
                    }
                    return Ok(());
                }
            }
            conf.save()?;
        }
    }
    Ok(())
}
