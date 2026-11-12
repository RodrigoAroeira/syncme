use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, ensure};

pub fn rsync(src: &str, dest: &str) -> Result<()> {
    let status = Command::new("rsync")
        .args(["-avz", "--mkpath", "--progress", src, dest])
        .status()
        .context("Failed to execute rsync.")?;

    ensure!(status.success(), "rsync failed with status: {status}");
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

    #[test]
    fn remote_path_inside_home() {
        let home = Path::new("/home/user");
        let file = Path::new("/home/user/.config/app/conf.json");
        assert_eq!(remote_path(file, home), Path::new(".config/app/conf.json"));
    }

    #[test]
    fn remote_path_outside_home() {
        let home = Path::new("/home/user");
        let file = Path::new("/etc/hosts");
        assert_eq!(remote_path(file, home), Path::new("/etc/hosts"));
    }

    fn get_home() -> &'static Path {
        "/home/user".as_ref()
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
}
