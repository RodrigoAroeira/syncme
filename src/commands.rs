use std::io;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use path_absolutize::Absolutize;

use crate::cli::MachineCmd as MCmd;
use crate::config::{Config, ConfigError as CE};
use crate::sync;

pub fn init(main_machine: &str) -> Result<()> {
    match Config::load() {
        Ok(_) => bail!("config already exists"),
        Err(CE::Io(e)) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Config::new(main_machine.to_string()).save()?;
    Ok(())
}

pub fn print(conf: &Config) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(conf)?);
    Ok(())
}

pub fn push(conf: &Config, home: &Path) -> Result<()> {
    for file in conf.paths() {
        let (local, remote) = local_and_remote(file, home)?;
        if conf.this() != conf.main() {
            if let Err(e) = sync::rsync(local, &sync::remote_spec(conf.main(), &remote)) {
                eprintln!("{e}");
            }
            continue;
        }

        for machine in conf.machines() {
            if let Err(e) = sync::rsync(local, &sync::remote_spec(machine, &remote)) {
                eprintln!("{e}");
            }
        }
    }
    Ok(())
}

pub fn pull(conf: &Config, machine: &str, home: &Path) -> Result<()> {
    ensure!(
        conf.this() != conf.main(),
        "Nothing to do: already on main machine",
    );

    ensure!(
        conf.this() != machine,
        "Nothing to do: target is current machine",
    );

    ensure!(
        conf.machines().iter().any(|m| m == machine),
        "target machine not found in configured machines",
    );

    for file in conf.paths() {
        if let Err(e) = (|| -> Result<()> {
            let (local, remote) = local_and_remote(file, home)?;
            sync::rsync(&sync::remote_spec(machine, &remote), local)?;
            Ok(())
        })() {
            eprintln!("{e}");
        }
    }
    Ok(())
}

pub fn prune(conf: &mut Config, verbose: bool) -> Result<()> {
    let removed = conf.prune_paths();
    conf.save()?;
    println!("pruned {} missing path(s)", removed.len());
    if verbose {
        for path in removed {
            println!("\t{}", path.display());
        }
    }
    Ok(())
}

pub fn track(conf: &mut Config, path: &Path) -> Result<()> {
    conf.add_path(path.absolutize()?);
    Ok(conf.save()?)
}

pub fn untrack(conf: &mut Config, path: &Path) -> Result<()> {
    conf.untrack_path(path.absolutize()?)?;
    Ok(conf.save()?)
}

pub fn machine(conf: &mut Config, cmd: MCmd) -> Result<()> {
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
    Ok(conf.save()?)
}

fn local_and_remote<'a>(file: &'a Path, home: &Path) -> Result<(&'a str, String)> {
    let local = file.to_str().context("invalid UTF-8 path")?;
    let remote = sync::remote_path(file, home)
        .to_str()
        .context("invalid UTF-8 path")?
        .to_owned();
    ensure!(!remote.is_empty(), "refusing to sync home directory itself");
    Ok((local, remote))
}
