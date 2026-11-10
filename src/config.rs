use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_FILENAME: &str = concat!(env!("CARGO_PKG_NAME"), "-conf.json");

#[derive(Serialize, Deserialize)]
pub struct Config {
    main_machine: String,
    connected_machines: Vec<String>,
    paths: Vec<PathBuf>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = get_path()?;
        let bytes = std::fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = get_path()?;
        let obj = serde_json::to_string_pretty(self).context("generate json from Config")?;

        let tmp = path.with_added_extension(".tmp");
        std::fs::write(&tmp, obj).context("Write to temp file")?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("Rename '{}' to '{}'", tmp.display(), path.display()))
    }

    pub fn add_path<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();

        if !self.paths.contains(&path) {
            self.paths.push(path)
        }
    }
}

fn get_path() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join(CONFIG_FILENAME))
        .context("get config dir")
}
