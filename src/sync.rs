use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use clap::Args;

use crate::util;

const SSH_COMMAND: &str = "ssh -o BatchMode=yes -o ConnectTimeout=10 \
    -o ConnectionAttempts=1 -o ServerAliveInterval=15 -o ServerAliveCountMax=3";

/// Options controlling how rsync transfers are performed.
#[derive(Args, Default, Clone, Copy)]
#[command(next_help_heading = "Sync options")]
pub struct Options {
    /// Simulate transfers without changing files.
    #[arg(short = 'n', long, global = true)]
    pub dry_run: bool,

    /// Refuse to delete more than N files from a destination.
    #[arg(long, global = true, value_name = "N")]
    pub max_delete: Option<u32>,

    /// Show per-file transfer progress.
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("could not run rsync")]
    Spawn(#[from] std::io::Error),
    #[error("rsync failed for '{src}' -> {dest} ({status})")]
    Failed {
        src: String,
        dest: String,
        status: ExitStatus,
    },
}

impl SyncError {
    pub fn is_fatal(&self) -> bool {
        matches!(self,
        Self::Failed { status, .. } if status.code() == Some(255))
    }
}

pub type Result<T> = std::result::Result<T, SyncError>;

fn rsync_args(src: &str, dest: &str, opts: &Options) -> Vec<String> {
    let mut args = [
        "-avz",
        "--delete",
        "--mkpath",
        "--delay-updates",
        "-i",
        "-s",
        "--timeout=60",
        "-e",
        SSH_COMMAND,
    ]
    .map(String::from)
    .to_vec();

    if opts.dry_run {
        args.push("-n".into());
    }
    if let Some(max) = opts.max_delete {
        args.push(format!("--max-delete={max}"));
    }
    if opts.verbose {
        args.push("--progress".into());
    }

    args.push("--".into());
    args.push(src.into());
    args.push(dest.into());
    args
}

pub fn rsync(src: &str, dest: &str, opts: &Options) -> Result<()> {
    let status = Command::new("rsync")
        .args(rsync_args(src, dest, opts))
        .status()?;

    if !status.success() {
        return Err(SyncError::Failed {
            src: src.into(),
            dest: dest.into(),
            status,
        });
    }
    Ok(())
}

pub fn remote_spec(machine: &str, path: &str) -> String {
    format!("{machine}:{path}")
}

pub fn remote_path(file: &Path, home: &Path) -> Option<PathBuf> {
    let rel = file.strip_prefix(home).ok()?;

    Some(if rel.as_os_str().is_empty() {
        PathBuf::new()
    } else if file.to_string_lossy().ends_with('/') {
        // strip_prefix removes trailing slash somehow
        // it's important for rsync
        util::with_trailing_slash(rel)
    } else {
        rel.to_path_buf()
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::process::ExitStatusExt;

    use super::*;

    fn get_home() -> &'static Path {
        "/home/user".as_ref()
    }

    #[test]
    fn remote_path_is_home() {
        assert_eq!(
            remote_path(&get_home().join(""), get_home()),
            Some(PathBuf::new())
        );
    }

    #[test]
    fn remote_path_inside_home() {
        let home = get_home();
        let file = home.join(".config/app/conf.json");
        assert_eq!(
            remote_path(&file, home),
            Some(PathBuf::from(".config/app/conf.json"))
        );
    }

    #[test]
    fn remote_path_outside_home() {
        let home = get_home();
        let file = Path::new("/etc/hosts");
        assert_eq!(remote_path(file, home), None);
    }

    #[test]
    fn trailing_slash_inside_home() {
        let home = get_home();
        let file = home.join(".config/");
        assert_eq!(remote_path(&file, home).unwrap().to_str(), Some(".config/"));
    }

    #[test]
    fn trailing_slash_outside_home() {
        let home = get_home();
        let file = "/foo/bar/".as_ref();
        assert_eq!(remote_path(file, home), None);
    }

    #[test]
    fn rsync_reports_a_failed_status() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");
        let dest = dir.path().join("dest");

        let err = rsync(
            missing.to_str().unwrap(),
            dest.to_str().unwrap(),
            &Options::default(),
        )
        .unwrap_err();

        assert!(
            matches!(err, SyncError::Failed { .. }),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn dirs_overwritten() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir1_path = dir1.path();
        std::fs::write(dir1_path.join("foo"), "foo").unwrap();

        let dir2 = tempfile::tempdir().unwrap();
        let dir2_path = dir2.path();
        std::fs::write(dir2_path.join("stale"), "stale").unwrap();

        let src = format!("{}/", dir1_path.display());
        let dest = dir2_path.to_str().unwrap();
        rsync(&src, dest, &Options::default()).unwrap();
        rsync(&src, dest, &Options::default()).unwrap();

        assert!(dir2_path.join("foo").exists());
        assert!(!dir2_path.join("stale").exists());
        assert!(!dir2_path.join(dir1_path.file_name().unwrap()).exists());
    }

    #[test]
    fn remote_path_sibling_prefix_is_outside() {
        let file = Path::new("/home/user2/.bashrc");
        assert_eq!(remote_path(file, get_home()).as_deref(), None)
    }

    #[test]
    fn remote_spec_joins_machine_and_path() {
        assert_eq!(remote_spec("nas", ".bashrc"), "nas:.bashrc");
        assert_eq!(remote_spec("nas", "/etc/hostname"), "nas:/etc/hostname");
    }

    #[test]
    fn rsync_args_terminate_options_before_paths() {
        assert_eq!(
            rsync_args("/home/user/.bashrc", "host:.bashrc", &Options::default()),
            [
                "-avz",
                "--delete",
                "--mkpath",
                "--delay-updates",
                "-i",
                "-s",
                "--timeout=60",
                "-e",
                SSH_COMMAND,
                "--",
                "/home/user/.bashrc",
                "host:.bashrc",
            ]
        );
    }

    #[test]
    fn rsync_args_apply_safety_options() {
        let opts = Options {
            dry_run: true,
            max_delete: Some(3),
            verbose: true,
        };

        assert_eq!(
            rsync_args("/home/user/.bashrc", "host:.bashrc", &opts),
            [
                "-avz",
                "--delete",
                "--mkpath",
                "--delay-updates",
                "-i",
                "-s",
                "--timeout=60",
                "-e",
                SSH_COMMAND,
                "-n",
                "--max-delete=3",
                "--progress",
                "--",
                "/home/user/.bashrc",
                "host:.bashrc",
            ]
        );
    }

    #[test]
    fn dry_run_does_not_change_destination() {
        let src_dir = tempfile::tempdir().unwrap();
        std::fs::write(src_dir.path().join("new"), "new").unwrap();

        let dest_dir = tempfile::tempdir().unwrap();
        std::fs::write(dest_dir.path().join("stale"), "stale").unwrap();

        let src = format!("{}/", src_dir.path().display());
        let dest = dest_dir.path().to_str().unwrap();
        let opts = Options {
            dry_run: true,
            ..Options::default()
        };

        rsync(&src, dest, &opts).unwrap();

        assert!(dest_dir.path().join("stale").exists());
        assert!(!dest_dir.path().join("new").exists());
    }

    #[test]
    fn ssh_fatal_status_is_detected() {
        let err = SyncError::Failed {
            src: "src".into(),
            dest: "dest".into(),
            status: std::process::ExitStatus::from_raw(255 << 8),
        };

        assert!(err.is_fatal());
    }
}
