use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use crate::util;

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

pub type Result<T> = std::result::Result<T, SyncError>;

fn rsync_args(src: &str, dest: &str) -> Vec<String> {
    [
        "-avz",
        "--delete",
        "--mkpath",
        "--delay-updates",
        "-i",
        "--",
        src,
        dest,
    ]
    .map(String::from)
    .to_vec()
}

pub fn rsync(src: &str, dest: &str) -> Result<()> {
    let status = Command::new("rsync").args(rsync_args(src, dest)).status()?;

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

        let err = rsync(missing.to_str().unwrap(), dest.to_str().unwrap()).unwrap_err();

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
        rsync(&src, dest).unwrap();
        rsync(&src, dest).unwrap();

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
            rsync_args("/home/user/.bashrc", "host:.bashrc"),
            [
                "-avz",
                "--delete",
                "--mkpath",
                "--delay-updates",
                "-i",
                "--",
                "/home/user/.bashrc",
                "host:.bashrc",
            ]
        );
    }
}
