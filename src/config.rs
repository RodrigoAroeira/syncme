use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_FILENAME: &str = concat!(env!("CARGO_PKG_NAME"), "-conf.json");

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    // TODO: Find a way to not rely on explicit local machine identifier this
    this_machine: String,
    main_machine: String,
    connected_machines: Vec<String>,
    paths: Vec<PathBuf>,
}

impl Config {
    pub fn new(main_machine: String) -> Self {
        let this_machine = std::fs::read_to_string("/etc/hostname")
            .as_deref()
            .map(str::trim)
            .map(String::from)
            .unwrap_or_default();
        Self {
            main_machine,
            this_machine,
            ..Default::default()
        }
    }
    pub fn load() -> Result<Self> {
        Self::load_from(get_path()?)
    }

    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn this(&self) -> &str {
        &self.this_machine
    }

    pub fn main(&self) -> &str {
        &self.main_machine
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub fn machines(&self) -> &[String] {
        &self.connected_machines
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(get_path()?)
    }

    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let obj = serde_json::to_string_pretty(self).context("generate json from Config")?;

        let tmp = path.with_added_extension(".tmp");
        std::fs::write(&tmp, obj).context("Write to temp file")?;
        std::fs::rename(&tmp, path)
            .with_context(|| format!("Rename '{}' to '{}'", tmp.display(), path.display()))
    }

    pub fn add_path<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();

        if !self.paths.contains(&path) {
            self.paths.push(path)
        }
    }

    pub fn untrack_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        self.paths
            .iter()
            .position(|p| p == path)
            .map(|idx| _ = self.paths.remove(idx))
            .with_context(|| format!("Path '{}' is not being tracked", path.display()))
    }
}

fn get_path() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join(CONFIG_FILENAME))
        .context("get config dir")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_save_and_load() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let config_file = dir.path().join("syncme-conf.json");

        let mut config = Config::new("primary_node".to_string());
        config.add_path("/home/user/.bashrc");
        config.save_to(&config_file)?;

        let loaded = Config::load_from(&config_file)?;
        assert_eq!(loaded.main(), "primary_node");
        assert_eq!(loaded.paths(), &[PathBuf::from("/home/user/.bashrc")]);

        Ok(())
    }

    #[test]
    fn add_path_deduplication() {
        let mut config = Config::new("primary_node".to_string());
        config.add_path("/home/user/.bashrc");
        config.add_path("/home/user/.bashrc");

        assert_eq!(config.paths().len(), 1);
        assert_eq!(config.paths()[0], Path::new("/home/user/.bashrc"));
    }

    #[test]
    fn untrack_path() {
        let mut config = Config::new("primary_node".to_string());
        let path = "/home/user/.bashrc";
        config.add_path(path);
        assert!(config.untrack_path(path).is_ok());
        assert_eq!(config.paths().len(), 0);
        assert!(config.untrack_path(path).is_err());
    }
}
