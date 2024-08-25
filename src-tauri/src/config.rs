use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct ConfigValues {
  pub user: String,
  pub primary_display: Vec<String>,
}

pub type Config = Arc<ConfigValues>;

pub fn load(path: &Option<PathBuf>) -> Result<Config> {
  if let Some(path) = path {
    let raw = std::fs::read_to_string(path)?;
    let parsed: ConfigValues = toml::from_str(&raw)?;
    println!("Using provided config file: {}", path.to_string_lossy());
    return Ok(Arc::new(parsed));
  }

  if let Some(config_dir) = dirs::config_dir() {
    let path = config_dir.join("dash2/config.toml");
    if path.exists() {
      let raw = std::fs::read_to_string(&path)?;
      let parsed: ConfigValues = toml::from_str(&raw)?;
      println!("Using config file: {}", path.to_string_lossy());
      return Ok(Arc::new(parsed));
    }
  }

  let etc_path = PathBuf::from("/etc/dash2/config.toml");
  if etc_path.exists() {
    let raw = std::fs::read_to_string(&etc_path)?;
    let parsed: ConfigValues = toml::from_str(&raw)?;
    println!("Using config file: {}", etc_path.to_string_lossy());
    return Ok(Arc::new(parsed));
  }

  anyhow::bail!("No config file found");
}
