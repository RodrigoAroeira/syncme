use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::util;

const CONFIG_FILENAME: &str = concat!(env!("CARGO_PKG_NAME"), "-conf.json");

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not locate the config directory")]
    ConfigDir,

    #[error("config I/O failed")]
    Io(#[from] std::io::Error),

    #[error("config JSON failed")]
    Json(#[from] serde_json::Error),

    #[error("path '{}' is not being tracked", .0.display())]
    PathNotTracked(PathBuf),

    #[error("machine '{0}' is not in the list")]
    MachineNotListed(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

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
        let bytes = fs_err::read(path)?;
        let mut config: Self = serde_json::from_slice(&bytes)?;
        config.normalize_paths();
        Ok(config)
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
        let obj = serde_json::to_string_pretty(self)?;

        let tmp = path.with_added_extension(".tmp");
        fs_err::write(&tmp, obj)?;
        fs_err::rename(&tmp, path)?;
        Ok(())
    }

    pub fn add_path<P: AsRef<Path>>(&mut self, path: P) {
        let path = normalize_path(path.as_ref()).to_path_buf();

        if !self.paths.contains(&path) {
            self.paths.push(path)
        }
    }

    pub fn untrack_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        remove_first(
            &mut self.paths,
            |p| p == path,
            || ConfigError::PathNotTracked(path.into()),
        )
    }

    pub fn add_machine(&mut self, name: String) {
        let name = name.trim().to_string();
        if !self.connected_machines.contains(&name) {
            self.connected_machines.push(name);
        }
    }

    pub fn rm_machine(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        remove_first(
            &mut self.connected_machines,
            |m| m.as_str() == name,
            || ConfigError::MachineNotListed(name.to_string()),
        )
    }

    pub fn prune_paths(&mut self) -> Vec<PathBuf> {
        let (kept, removed) = self
            .paths
            .drain(..)
            .partition(|p| std::fs::symlink_metadata(p).is_ok());

        self.paths = kept;
        self.normalize_paths();
        removed
    }

    fn normalize_paths(&mut self) {
        for path in self.paths.iter_mut() {
            *path = normalize_path(path);
        }
    }
}

fn remove_first<T>(
    items: &mut Vec<T>,
    pred: impl Fn(&T) -> bool,
    err: impl FnOnce() -> ConfigError,
) -> Result<()> {
    let idx = items.iter().position(pred).ok_or_else(err)?;
    items.remove(idx);
    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let normalized: PathBuf = path.components().collect();

    let Ok(meta) = std::fs::symlink_metadata(&normalized) else {
        return path.into();
    };

    if meta.is_dir() {
        util::with_trailing_slash(normalized)
    } else {
        normalized
    }
}

fn get_path() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join(CONFIG_FILENAME))
        .ok_or(ConfigError::ConfigDir)
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

    #[test]
    fn add_machine() {
        let mut config = Config::new("primary_node".to_string());
        let name = "machine".to_string();
        config.add_machine(name.clone());
        assert!(config.machines().contains(&name));
        config.add_machine(name);
        assert_eq!(config.machines().len(), 1);
    }

    #[test]
    fn rm_machine() {
        let mut config = Config::new("primary_node".to_string());
        let name = "machine".to_string();
        config.add_machine(name.clone());
        assert!(config.rm_machine(&name).is_ok());
    }

    #[test]
    fn normalize_paths_dirs_only() {
        let dir = tempfile::tempdir().unwrap();
        let dirpath = dir.path();
        let subdir = dirpath.join("sub");
        let file = dirpath.join("file");
        let missing = dirpath.join("missing");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(&file, "").unwrap();

        assert!(normalize_path(&subdir).to_string_lossy().ends_with('/'));
        assert!(!normalize_path(&file).to_string_lossy().ends_with('/'));
        assert_eq!(normalize_path(&missing), missing);
    }

    #[test]
    fn load_normalizes_existing_paths() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir)?;
        let config_file = dir.path().join("syncme-conf.json");
        let json = format!(
            r#"{{"this_machine":"m","main_machine":"m","connected_machines":[],"paths":["{}"]}}"#,
            subdir.display()
        );
        std::fs::write(&config_file, json)?;

        let loaded = Config::load_from(&config_file)?;
        assert!(loaded.paths()[0].to_string_lossy().ends_with('/'));

        Ok(())
    }

    #[test]
    fn save_writes_canonical_paths() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir)?;
        let config_file = dir.path().join("syncme-conf.json");

        let mut config = Config::new("primary_node".to_string());
        config.add_path(&subdir);
        config.save_to(&config_file)?;

        let raw = std::fs::read_to_string(&config_file)?;
        assert!(raw.contains(&format!("\"{}/\"", subdir.display())));

        Ok(())
    }

    #[test]
    fn prune_paths_removes_missing_and_normalizes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let subdir = dir.path().join("subdir");
        let file = dir.path().join("file");
        let missing = dir.path().join("missing");
        std::fs::create_dir(&subdir)?;
        std::fs::write(&file, "x")?;

        let mut config = Config::new("primary_node".to_string());
        config.add_path(&subdir);
        config.add_path(&file);
        config.add_path(&missing);

        let removed = config.prune_paths();
        assert_eq!(removed, vec![missing]);
        assert_eq!(config.paths().len(), 2);
        assert!(config.paths()[0].to_string_lossy().ends_with('/'));

        Ok(())
    }
}
