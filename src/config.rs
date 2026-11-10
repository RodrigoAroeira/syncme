use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub main_machine: String,
    pub connected_machines: Vec<String>,
    pub files: Vec<PathBuf>,
}

pub fn get_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}
