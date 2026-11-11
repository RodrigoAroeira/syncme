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
