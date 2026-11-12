use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

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

pub fn rsync(src: &str, dest: &str) -> Result<()> {
    let status = Command::new("rsync")
        .args(["-avz", "--delete", "--mkpath", "--progress", src, dest])
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

pub fn remote_path(file: &Path, home: &Path) -> PathBuf {
    let Ok(rel) = file.strip_prefix(home) else {
        return file.to_path_buf();
    };

    // strip_prefix removes trailing slash somehow
    // it's important for rsync
    if file.to_string_lossy().ends_with('/') {
        let mut os = rel.as_os_str().to_os_string();
        os.push("/");
        PathBuf::from(os)
    } else {
        rel.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_home() -> &'static Path {
        "/home/user".as_ref()
    }

    #[test]
    fn remote_path_inside_home() {
        let home = get_home();
        let file = home.join(".config/app/conf.json");
        assert_eq!(remote_path(&file, home), Path::new(".config/app/conf.json"));
    }

    #[test]
    fn remote_path_outside_home() {
        let home = get_home();
        let file = Path::new("/etc/hosts");
        assert_eq!(remote_path(file, home), Path::new("/etc/hosts"));
    }

    #[test]
    fn trailing_slash_inside_home() {
        let home = get_home();
        let file = home.join(".config/");
        assert_eq!(remote_path(&file, home).to_str(), Some(".config/"));
    }

    #[test]
    fn trailing_slash_outside_home() {
        let home = get_home();
        let file = "/foo/bar/".as_ref();
        assert_eq!(remote_path(file, home).to_str(), Some("/foo/bar/"));
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
}
