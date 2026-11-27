use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};

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

fn apply_batch<T, R: PartialEq>(
    items: &[T],
    action: &str,
    noun: &str,
    label: impl Fn(&T) -> String,
    mut check: impl FnMut(&T) -> Result<R>,
    mut apply: impl FnMut(R) -> Result<()>,
) -> Result<()> {
    ensure!(!items.is_empty(), "no {noun} provided");

    let mut checked = Vec::new();
    let mut failures = 0;

    for item in items {
        match check(item) {
            Ok(value) if !checked.contains(&value) => checked.push(value),
            Ok(_) => {}
            Err(e) => {
                eprintln!("[{action}] {}: {e:#}", label(item));
                failures += 1;
            }
        }
    }

    ensure!(failures == 0, "{failures} {noun} failed validation");

    for value in checked {
        apply(value)?;
    }
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

pub fn track(conf: &mut Config, paths: &[PathBuf], home: &Path) -> Result<()> {
    apply_batch(
        paths,
        "track",
        "path(s)",
        |path| path.display().to_string(),
        |path| checked_track_path(path, home),
        |path| {
            conf.add_path(path);
            Ok(())
        },
    )?;
    conf.save()?;
    Ok(())
}

pub fn untrack(conf: &mut Config, paths: &[PathBuf]) -> Result<()> {
    let tracked = conf.paths().to_vec();
    apply_batch(
        paths,
        "untrack",
        "path(s)",
        |path| path.display().to_string(),
        |path| checked_untrack_path(&tracked, path),
        |path| conf.untrack_path(path).map_err(Into::into),
    )?;
    conf.save()?;
    Ok(())
}

pub fn machine(conf: &mut Config, cmd: MCmd) -> Result<()> {
    match cmd {
        MCmd::Add { names } => {
            apply_batch(
                &names,
                "machine add",
                "machine name(s)",
                |name| name.clone(),
                |name| validate_machine_name(name).map(|()| name.clone()),
                |name| {
                    conf.add_machine(name);
                    Ok(())
                },
            )?;
        }
        MCmd::Remove { names } => {
            let machines = conf.machines().to_vec();
            apply_batch(
                &names,
                "machine remove",
                "machine name(s)",
                |name| name.clone(),
                |name| checked_remove_machine(&machines, name),
                |name| conf.rm_machine(&name).map_err(Into::into),
            )?;
        }
        MCmd::List => {
            for m in conf.machines() {
                println!("- {m}");
            }
            return Ok(());
        }
    }
    Ok(conf.save()?)
}

fn validate_machine_name(name: &str) -> Result<()> {
    ensure!(!name.is_empty(), "machine name must not be empty");
    ensure!(!name.contains(':'), "machine name must not contain ':'");
    ensure!(
        !name.contains(char::is_whitespace),
        "machine name must not contain whitespaces"
    );
    ensure!(!name.starts_with('-'), "forbidden pattern: leading '-'");
    Ok(())
}

fn checked_remove_machine(machines: &[String], name: &str) -> Result<String> {
    let name = name.trim().to_string();
    ensure!(
        machines.iter().any(|m| m.as_str() == name),
        "machine '{name}' is not in the list"
    );
    Ok(name)
}

fn checked_track_path(path: &Path, home: &Path) -> Result<PathBuf> {
    let absolute = path.absolutize()?.into_owned();
    validate_tracked_path(&absolute, home)?;
    Ok(absolute)
}

fn validate_tracked_path(path: &Path, home: &Path) -> Result<()> {
    ensure!(path != home, "refusing to track home directory itself");
    ensure!(
        path.starts_with(home),
        "refusing to track {}: outside the home directory",
        path.display()
    );
    Ok(())
}

fn checked_untrack_path(tracked: &[PathBuf], path: &Path) -> Result<PathBuf> {
    let absolute = path.absolutize()?.into_owned();
    ensure!(
        tracked
            .iter()
            .any(|tracked| tracked.as_path() == absolute.as_path()),
        "path '{}' is not being tracked",
        absolute.display()
    );
    Ok(absolute)
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
    fn validate_tracked_path_accepts_inside_home() {
        assert!(validate_tracked_path(Path::new("/home/user/.config/"), home()).is_ok());
    }

    #[test]
    fn validate_tracked_path_rejects_home_itself() {
        let err = validate_tracked_path(home(), home()).unwrap_err();

        assert!(err.to_string().contains("home directory itself"));
    }

    #[test]
    fn validate_tracked_path_rejects_outside_home() {
        let err = validate_tracked_path(Path::new("/etc/hosts"), home()).unwrap_err();

        assert!(err.to_string().contains("outside the home directory"));
    }

    #[test]
    fn validate_machine_name_accepts_simple_and_user_host() {
        assert!(validate_machine_name("laptop").is_ok());
        assert!(validate_machine_name("user@host").is_ok());
    }

    #[test]
    fn validate_machine_name_rejects_unsafe_names() {
        for name in ["", "-host", "a:b", "a b"] {
            assert!(validate_machine_name(name).is_err(), "accepted {name}");
        }
    }

    #[test]
    fn checked_remove_machine_accepts_listed_name() {
        let conf = get_config("main", "laptop", &["main", "laptop"]);

        assert_eq!(
            checked_remove_machine(conf.machines(), "laptop").unwrap(),
            "laptop"
        );
    }

    #[test]
    fn checked_remove_machine_rejects_unlisted_name() {
        let conf = get_config("main", "laptop", &["main", "laptop"]);
        let err = checked_remove_machine(conf.machines(), "desktop").unwrap_err();

        assert!(err.to_string().contains("not in the list"));
    }

    #[test]
    fn checked_untrack_path_accepts_tracked_path() {
        let mut conf = get_config("main", "laptop", &[]);
        conf.add_path("/home/user/.bashrc");

        let checked = checked_untrack_path(conf.paths(), Path::new("/home/user/.bashrc")).unwrap();

        assert_eq!(checked, PathBuf::from("/home/user/.bashrc"));
    }

    #[test]
    fn checked_untrack_path_rejects_untracked_path() {
        let conf = get_config("main", "laptop", &[]);
        let err = checked_untrack_path(conf.paths(), Path::new("/home/user/.bashrc")).unwrap_err();

        assert!(err.to_string().contains("not being tracked"));
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
