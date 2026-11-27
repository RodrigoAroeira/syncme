use std::fmt::Display;
use std::io;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use path_absolutize::Absolutize;

use crate::cli::MachineCmd as MCmd;
use crate::config::{Config, ConfigError as CE};
use crate::sync;

fn report_failure(machine: &str, file: &Path, err: impl Display) {
    eprintln!("[{machine}] {}: {err:#}", file.display())
}

fn fail_on_errors(failures: usize) -> Result<()> {
    ensure!(failures == 0, "{failures} transfer(s) failed");
    Ok(())
}

#[derive(Clone, Copy)]
enum Direction {
    Push,
    Pull,
}

impl Direction {
    fn endpoints<'a>(self, local: &'a str, remote: &'a str) -> (&'a str, &'a str) {
        match self {
            Self::Push => (local, remote),
            Self::Pull => (remote, local),
        }
    }

    fn stopped_message(self) -> &'static str {
        match self {
            Self::Push => "to this machine",
            Self::Pull => "from this machine",
        }
    }
}

pub fn init(main_machine: &str) -> Result<()> {
    ensure!(!main_machine.is_empty(), "main_machine must not be empty");

    match Config::load() {
        Ok(_) => bail!("config already exists"),
        Err(CE::Io(e)) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Config::new(main_machine.to_string()).save()?;
    Ok(())
}

pub fn print(conf: &Config) -> serde_json::Result<()> {
    println!("{}", serde_json::to_string_pretty(conf)?);
    Ok(())
}

pub fn push(conf: &Config, home: &Path, opts: &sync::Options) -> Result<()> {
    let targets = push_targets(conf);
    sync_paths(conf, home, opts, &targets, Direction::Push)
}

pub fn pull(conf: &Config, machine: &str, home: &Path, opts: &sync::Options) -> Result<()> {
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

    sync_paths(conf, home, opts, &[machine], Direction::Pull)
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

pub fn track(conf: &mut Config, path: &Path, home: &Path) -> Result<()> {
    let path = path.absolutize()?;
    ensure!(
        path.as_ref() != home,
        "refusing to track home directory itself"
    );
    ensure!(
        path.starts_with(home),
        "refusing to track {}: outside the home directory",
        path.display()
    );
    conf.add_path(path);
    Ok(conf.save()?)
}

pub fn untrack(conf: &mut Config, path: &Path) -> Result<()> {
    conf.untrack_path(path.absolutize()?)?;
    Ok(conf.save()?)
}

pub fn machine(conf: &mut Config, cmd: MCmd) -> Result<()> {
    match cmd {
        MCmd::Add { name } => {
            ensure!(!name.is_empty(), "machine name must not be empty");
            ensure!(!name.contains(':'), "machine name must not contain ':'");
            ensure!(
                !name.contains(char::is_whitespace),
                "machine name must not contain whitespaces"
            );
            ensure!(!name.starts_with('-'), "forbidden pattern: leading '-'");
            conf.add_machine(name);
        }
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
        .with_context(|| {
            format!(
                "refusing to sync {}: outside the home directory",
                file.display()
            )
        })?
        .to_str()
        .context("invalid UTF-8 path")?
        .to_owned();
    ensure!(!remote.is_empty(), "refusing to sync home directory itself");
    ensure!(
        !Path::new(&remote).starts_with(".."),
        "refusing to sync {}: path escapes the home directory",
        file.display()
    );
    Ok((local, remote))
}

fn push_targets(conf: &Config) -> Vec<&str> {
    if conf.this() != conf.main() {
        vec![conf.main()]
    } else {
        conf.machines()
            .iter()
            .map(String::as_str)
            .filter(|m| *m != conf.this())
            .collect()
    }
}

fn sync_paths(
    conf: &Config,
    home: &Path,
    opts: &sync::Options,
    targets: &[&str],
    direction: Direction,
) -> Result<()> {
    let mut failures = 0;

    for &machine in targets {
        for file in conf.paths() {
            let (local, remote) = match local_and_remote(file, home) {
                Ok(paths) => paths,
                Err(e) => {
                    report_failure(machine, file, e);
                    failures += 1;
                    continue;
                }
            };

            let remote_spec = sync::remote_spec(machine, &remote);
            let (src, dest) = direction.endpoints(local, &remote_spec);

            match sync::rsync(src, dest, opts) {
                Ok(()) => {}
                Err(e) => {
                    let fatal = e.is_fatal();
                    report_failure(machine, file, e);
                    failures += 1;

                    if fatal {
                        eprintln!(
                            "[{machine}] stopping transfers {} this machine",
                            direction.stopped_message()
                        );
                        break;
                    }
                }
            }
        }
    }

    fail_on_errors(failures)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> &'static Path {
        "/home/user".as_ref()
    }

    #[test]
    fn local_and_remote_maps_inside_home() {
        let file = home().join(".config/nvim/");

        let (local, remote) = local_and_remote(&file, home()).unwrap();

        assert_eq!(local, "/home/user/.config/nvim/");
        assert_eq!(remote, ".config/nvim/");
    }

    #[test]
    fn local_and_remote_rejects_home() {
        // join("") appends the trailing slash
        let file = home().join("");

        let err = local_and_remote(&file, home()).unwrap_err();

        assert!(err.to_string().contains("refusing to sync home directory"));
    }

    #[test]
    fn local_and_remote_rejects_non_utf8_paths() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let raw = OsStr::from_bytes(b"/home/user/\xff");
        let file = Path::new(raw);

        let err = local_and_remote(file, home()).unwrap_err();

        assert!(err.to_string().contains("invalid UTF-8 path"));
    }

    #[test]
    fn local_and_remote_rejects_parent_dir_escape() {
        let file = home().join("../etc/hosts");
        let err = local_and_remote(&file, home()).unwrap_err();

        assert!(err.to_string().contains("path escapes the home directory"));
    }

    #[test]
    fn local_and_remote_allows_dot_dot_name() {
        let file = home().join("..foo");
        let (_, remote) = local_and_remote(&file, home()).unwrap();

        assert_eq!(remote, "..foo");
    }

    fn get_config(main: &str, this: &str, machines: &[&str]) -> Config {
        serde_json::from_value(serde_json::json!({
            "this_machine": this,
            "main_machine": main,
            "connected_machines": machines,
        }))
        .expect("valid test config")
    }

    #[test]
    fn test_push_target_from_main() {
        let conf = get_config("main", "main", &["main", "laptop", "desktop"]);
        assert_eq!(push_targets(&conf), ["laptop", "desktop"])
    }

    #[test]
    fn test_push_target_from_peer() {
        let conf = get_config("main", "laptop", &["main", "laptop", "desktop"]);
        assert_eq!(push_targets(&conf), ["main"])
    }
}
