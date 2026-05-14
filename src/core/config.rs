//! Persistent application configuration backed by a TOML file.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::types::{DeviceType, WorkspaceConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    pub app: AppConfig,
    pub device: DeviceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub last_directory: String,
    pub recent_files: Vec<String>,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub port: String,
    pub baud_rate: u32,
    pub device_type: DeviceType,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace: WorkspaceConfig::default(),
            app: AppConfig {
                last_directory: dirs::home_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                recent_files: vec![],
                theme: "dark".to_owned(),
            },
            device: DeviceConfig {
                port: String::new(),
                baud_rate: 115200,
                device_type: DeviceType::GrblLaser,
            },
        }
    }
}

impl Config {
    /// Load config from the default path, falling back to defaults on error.
    pub fn load() -> Self {
        match Self::load_from(&Self::default_path()) {
            Ok(cfg) => cfg,
            Err(_) => Self::default(),
        }
    }

    /// Save config to the default path.
    pub fn save(&self) {
        if let Err(e) = self.save_to(&Self::default_path()) {
            log::warn!("Failed to save config: {e}");
        }
    }

    pub fn add_recent_file(&mut self, path: &str) {
        self.app.recent_files.retain(|p| p != path);
        self.app.recent_files.insert(0, path.to_owned());
        self.app.recent_files.truncate(10);
    }

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("snart-laser")
            .join("config.toml")
    }

    fn load_from(path: &PathBuf) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    fn save_to(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}
