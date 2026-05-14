//! Persistent application configuration backed by a TOML file.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::types::{DeviceType, WorkspaceConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    pub app: AppConfig,
    pub device: DeviceConfig,
    pub visual: VisualConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub last_directory: String,
    pub recent_files: Vec<String>,
    pub theme: String,
}

/// Visual / display preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualConfig {
    /// Background colour of the area outside the work area (hex, e.g. "#222222").
    pub canvas_bg: String,
    /// Work area fill colour (hex, e.g. "#ffffff").
    pub workspace_bg: String,
    /// Grid line colour (hex).
    pub grid_color: String,
    /// Grid line opacity 0.0–1.0.
    pub grid_opacity: f32,
    /// Stroke width for vector shapes in screen pixels (independent of zoom).
    pub shape_stroke_px: f32,
    /// Selection highlight colour (hex).
    pub selection_color: String,
    /// Live tool-preview colour (hex).
    pub preview_color: String,
    /// Antialiasing on shapes.
    pub antialiasing: bool,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            canvas_bg:        "#222222".to_owned(),
            workspace_bg:     "#ffffff".to_owned(),
            grid_color:       "#000000".to_owned(),
            grid_opacity:     0.12,
            shape_stroke_px:  1.5,
            selection_color:  "#0078d4".to_owned(),
            preview_color:    "#ff6600".to_owned(),
            antialiasing:     true,
        }
    }
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
            visual: VisualConfig::default(),
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
