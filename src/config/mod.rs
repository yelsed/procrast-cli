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
            api_url: "https://procrastination-station-xwi7hrqi.on-forge.com/api".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Env var takes highest priority
        if let Ok(url) = std::env::var("PROCRAST_API_URL") {
            let config = Self { api_url: url };
            config.warn_if_insecure();
            return Ok(config);
        }

        // Then config file
        let path = Self::config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            config.warn_if_insecure();
            return Ok(config);
        }

        // Default
        Ok(Self::default())
    }

    fn warn_if_insecure(&self) {
        if !self.api_url.starts_with("https://") && !self.api_url.starts_with("http://localhost") {
            eprintln!(
                "WARNING: API URL does not use HTTPS ({}). Your auth token will be sent in cleartext.",
                self.api_url
            );
        }
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
