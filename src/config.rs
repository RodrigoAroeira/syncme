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

    #[error("peer '{0}' is not in the list")]
    PeerNotListed(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
pub enum Role {
    #[default]
    Peer,
    Hub,
}

impl Role {
    pub fn is_hub(self) -> bool {
        matches!(self, Self::Hub)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    role: Role,
    hub: Option<String>,
    peers: Vec<String>,
    paths: Vec<PathBuf>,
}

impl Config {
    pub fn new(role: Role, hub: Option<String>) -> Self {
        Self {
            role,
            hub,
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

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn hub(&self) -> Option<&str> {
        self.hub.as_deref()
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub fn peers(&self) -> &[String] {
        &self.peers
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

    pub fn add_peer(&mut self, name: String) {
        let name = name.trim().to_string();
        if !self.peers.contains(&name) {
            self.peers.push(name);
        }
    }

    pub fn rm_peer(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        remove_first(
            &mut self.peers,
            |m| m.as_str() == name,
            || ConfigError::PeerNotListed(name.to_string()),
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

    const HUB: &str = "hub-host";
    const BASHRC: &str = "/home/user/.bashrc";
    const PEER_NAME: &str = "peer-host";

    fn test_config() -> Config {
        Config::new(Role::Peer, Some(HUB.to_string()))
    }

    fn temp_config_file() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILENAME);
        (dir, path)
    }

    struct Fixture {
        subdir: PathBuf,
        file: PathBuf,
        missing: PathBuf,
    }

    fn fixture() -> (tempfile::TempDir, Fixture) {
        let dir = tempfile::tempdir().unwrap();
        let dirpath = dir.path();
        let subdir = dirpath.join("subdir");
        let file = dirpath.join("file");
        let missing = dirpath.join("missing");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(&file, "").unwrap();
        (
            dir,
            Fixture {
                subdir,
                file,
                missing,
            },
        )
    }

    #[test]
    fn config_save_and_load() -> Result<()> {
        let (_dir, config_file) = temp_config_file();

        let mut config = test_config();
        config.add_path(BASHRC);
        config.save_to(&config_file)?;

        let loaded = Config::load_from(&config_file)?;
        assert_eq!(loaded.role(), Role::Peer);
        assert_eq!(loaded.hub(), Some(HUB));
        assert_eq!(loaded.paths(), &[PathBuf::from(BASHRC)]);

        Ok(())
    }

    #[test]
    fn add_path_deduplication() {
        let mut config = test_config();
        config.add_path(BASHRC);
        config.add_path(BASHRC);

        assert_eq!(config.paths().len(), 1);
        assert_eq!(config.paths()[0], Path::new(BASHRC));
    }

    #[test]
    fn untrack_path() {
        let mut config = test_config();

        config.add_path(BASHRC);
        assert!(config.untrack_path(BASHRC).is_ok());
        assert_eq!(config.paths().len(), 0);
        assert!(config.untrack_path(BASHRC).is_err());
    }

    #[test]
    fn add_peer() {
        let mut config = test_config();
        let peer = PEER_NAME.to_string();
        config.add_peer(peer.clone());

        assert!(config.peers().contains(&peer));
        config.add_peer(peer);
        assert_eq!(config.peers().len(), 1);
    }

    #[test]
    fn rm_peer() {
        let mut config = test_config();
        config.add_peer(PEER_NAME.to_string());
        assert!(config.rm_peer(PEER_NAME).is_ok());
    }

    #[test]
    fn normalize_paths_dirs_only() {
        let (_dir, f) = fixture();

        assert!(normalize_path(&f.subdir).to_string_lossy().ends_with('/'));
        assert!(!normalize_path(&f.file).to_string_lossy().ends_with('/'));
        assert_eq!(normalize_path(&f.missing), f.missing);
    }

    #[test]
    fn load_normalizes_existing_paths() -> Result<()> {
        let (dir, config_file) = temp_config_file();
        let subdir = dir.path().join("subdir");

        std::fs::create_dir(&subdir)?;

        let json = format!(
            r#"{{"role":"peer","hub":"hub-host","peers":[],"paths":["{}"]}}"#,
            subdir.display()
        );
        std::fs::write(&config_file, json)?;

        let loaded = Config::load_from(&config_file)?;
        assert!(loaded.paths()[0].to_string_lossy().ends_with('/'));

        Ok(())
    }

    #[test]
    fn save_writes_canonical_paths() -> Result<()> {
        let (dir, config_file) = temp_config_file();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir)?;

        let mut config = test_config();
        config.add_path(&subdir);
        config.save_to(&config_file)?;

        let raw = std::fs::read_to_string(&config_file)?;
        assert!(raw.contains(&format!("\"{}/\"", subdir.display())));

        Ok(())
    }

    #[test]
    fn prune_paths_removes_missing_and_normalizes() -> Result<()> {
        let (_dir, f) = fixture();

        let mut config = test_config();
        config.add_path(&f.subdir);
        config.add_path(&f.file);
        config.add_path(&f.missing);

        let removed = config.prune_paths();
        assert_eq!(removed, vec![f.missing.clone()]);
        assert_eq!(config.paths().len(), 2);
        assert!(config.paths()[0].to_string_lossy().ends_with('/'));

        Ok(())
    }

    #[test]
    fn load_missing_file_returns_not_found() {
        let (_dir, config_file) = temp_config_file();

        match Config::load_from(&config_file) {
            Err(ConfigError::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
            other => panic!("expected Io(NotFound), got {:?}", other.err()),
        }
    }

    #[test]
    fn load_malformed_file_returns_json_error() {
        let (_dir, config_file) = temp_config_file();
        std::fs::write(&config_file, "{ malformed").unwrap();

        assert!(matches!(
            Config::load_from(&config_file),
            Err(ConfigError::Json(_))
        ))
    }

    #[test]
    fn load_ignores_unknown_fields() -> Result<()> {
        let (_dir, config_file) = temp_config_file();
        let json = r#"{"role":"peer","hub":"hub-host","peers":[],"paths":[],"future":"value"}"#;
        std::fs::write(&config_file, json)?;

        let loaded = Config::load_from(&config_file)?;
        assert_eq!(loaded.hub(), Some("hub-host"));

        Ok(())
    }

    #[test]
    fn role_defaults_to_peer() -> Result<()> {
        let (_dir, config_file) = temp_config_file();
        std::fs::write(&config_file, r#"{"paths":[]}"#)?;

        let loaded = Config::load_from(&config_file)?;
        assert_eq!(loaded.role(), Role::Peer);
        assert_eq!(loaded.hub(), None);

        Ok(())
    }

    #[test]
    fn save_writes_current_schema_only() -> Result<()> {
        let (_dir, config_file) = temp_config_file();

        test_config().save_to(&config_file)?;

        let raw = std::fs::read_to_string(&config_file)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        let mut keys: Vec<_> = value
            .as_object()
            .expect("config is an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();

        assert_eq!(keys, ["hub", "paths", "peers", "role"]);

        Ok(())
    }

    #[test]
    fn prune_keeps_symlinks() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let target = dir.path().join("target");
        std::fs::write(&target, "")?;
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link)?;
        let broken = dir.path().join("broken");
        std::os::unix::fs::symlink(dir.path().join("gone"), &broken)?;

        let mut config = test_config();
        config.add_path(&link);
        config.add_path(&broken);

        assert!(config.prune_paths().is_empty());
        assert_eq!(config.paths().len(), 2);

        Ok(())
    }

    #[test]
    fn save_and_load_roundtrip_unicode_paths() -> Result<()> {
        let (_dir, config_file) = temp_config_file();
        let weird = Path::new("/home/user/.config/naïve dir");

        let mut config = test_config();
        config.add_path(weird);
        config.save_to(&config_file)?;

        let loaded = Config::load_from(&config_file)?;
        assert_eq!(loaded.paths(), &[weird]);

        Ok(())
    }

    #[test]
    fn save_writes_pretty_json_and_cleans_up() -> Result<()> {
        let (_dir, config_file) = temp_config_file();
        let tmp = config_file.with_added_extension(".tmp");

        test_config().save_to(&config_file)?;

        assert!(config_file.exists());
        assert!(!tmp.exists(), "temporary file was left behind: {tmp:?}");
        let raw = std::fs::read_to_string(&config_file)?;
        assert!(raw.contains("\n  \""), "expected pretty JSON, got:\n{raw}");

        Ok(())
    }
}
