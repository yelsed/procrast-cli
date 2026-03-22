use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:8000/api".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Env var takes highest priority
        if let Ok(url) = std::env::var("PROCRAST_API_URL") {
            return Ok(Self { api_url: url });
        }

        // Then config file
        let path = Self::config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(config);
        }

        // Default
        Ok(Self::default())
    }

    pub fn config_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("procrast-cli");
        Ok(dir.join("config.toml"))
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
