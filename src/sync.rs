use std::path::Path;
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

pub fn remote_path<'a>(file: &'a Path, home: &Path) -> &'a Path {
    file.strip_prefix(home).unwrap_or(file)
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
}
