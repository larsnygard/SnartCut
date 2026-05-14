//! Persistent application configuration backed by a TOML file.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::types::{DeviceType, WorkspaceConfig};

// ---------------------------------------------------------------------------
// Key / mouse bindings
// ---------------------------------------------------------------------------

/// Identifies one user-configurable key action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingId {
    TempPan,
    ToolSelect,
    ToolPan,
    ToolRect,
    ToolEllipse,
    ToolLine,
    ToolPolyline,
    DeleteSelected,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

impl BindingId {
    pub fn label(self) -> &'static str {
        match self {
            BindingId::TempPan        => "Pan (hold)",
            BindingId::ToolSelect     => "Select tool",
            BindingId::ToolPan        => "Pan tool",
            BindingId::ToolRect       => "Rectangle tool",
            BindingId::ToolEllipse    => "Ellipse tool",
            BindingId::ToolLine       => "Line tool",
            BindingId::ToolPolyline   => "Polyline tool",
            BindingId::DeleteSelected => "Delete selected",
            BindingId::ZoomIn         => "Zoom in",
            BindingId::ZoomOut        => "Zoom out",
            BindingId::ZoomReset      => "Zoom reset",
        }
    }

    pub fn all() -> &'static [BindingId] {
        &[
            BindingId::TempPan,
            BindingId::ToolSelect,
            BindingId::ToolPan,
            BindingId::ToolRect,
            BindingId::ToolEllipse,
            BindingId::ToolLine,
            BindingId::ToolPolyline,
            BindingId::DeleteSelected,
            BindingId::ZoomIn,
            BindingId::ZoomOut,
            BindingId::ZoomReset,
        ]
    }
}

/// User-configurable keyboard bindings (stored as display strings, e.g. "Space", "r").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub temp_pan:        String,
    pub tool_select:     String,
    pub tool_pan:        String,
    pub tool_rect:       String,
    pub tool_ellipse:    String,
    pub tool_line:       String,
    pub tool_polyline:   String,
    pub delete_selected: String,
    pub zoom_in:         String,
    pub zoom_out:        String,
    pub zoom_reset:      String,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            temp_pan:        "Space".to_owned(),
            tool_select:     "s".to_owned(),
            tool_pan:        "p".to_owned(),
            tool_rect:       "r".to_owned(),
            tool_ellipse:    "e".to_owned(),
            tool_line:       "l".to_owned(),
            tool_polyline:   String::new(),
            delete_selected: "Delete".to_owned(),
            zoom_in:         "=".to_owned(),
            zoom_out:        "-".to_owned(),
            zoom_reset:      "0".to_owned(),
        }
    }
}

impl KeyBindings {
    pub fn get(&self, id: BindingId) -> &str {
        match id {
            BindingId::TempPan        => &self.temp_pan,
            BindingId::ToolSelect     => &self.tool_select,
            BindingId::ToolPan        => &self.tool_pan,
            BindingId::ToolRect       => &self.tool_rect,
            BindingId::ToolEllipse    => &self.tool_ellipse,
            BindingId::ToolLine       => &self.tool_line,
            BindingId::ToolPolyline   => &self.tool_polyline,
            BindingId::DeleteSelected => &self.delete_selected,
            BindingId::ZoomIn         => &self.zoom_in,
            BindingId::ZoomOut        => &self.zoom_out,
            BindingId::ZoomReset      => &self.zoom_reset,
        }
    }

    pub fn set(&mut self, id: BindingId, value: String) {
        match id {
            BindingId::TempPan        => self.temp_pan = value,
            BindingId::ToolSelect     => self.tool_select = value,
            BindingId::ToolPan        => self.tool_pan = value,
            BindingId::ToolRect       => self.tool_rect = value,
            BindingId::ToolEllipse    => self.tool_ellipse = value,
            BindingId::ToolLine       => self.tool_line = value,
            BindingId::ToolPolyline   => self.tool_polyline = value,
            BindingId::DeleteSelected => self.delete_selected = value,
            BindingId::ZoomIn         => self.zoom_in = value,
            BindingId::ZoomOut        => self.zoom_out = value,
            BindingId::ZoomReset      => self.zoom_reset = value,
        }
    }
}

/// What the scroll wheel does on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScrollAction {
    #[default]
    Zoom,
    PanVertical,
}

impl std::fmt::Display for ScrollAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ScrollAction::Zoom        => "Zoom",
            ScrollAction::PanVertical => "Pan vertical",
        })
    }
}

/// User-configurable mouse bindings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MouseBindings {
    /// What the scroll wheel does.
    pub scroll: ScrollAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    pub app: AppConfig,
    pub device: DeviceConfig,
    pub visual: VisualConfig,
    pub bindings: KeyBindings,
    pub mouse_bindings: MouseBindings,
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

/// A named device profile (port + baud + type + work area).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceProfile {
    pub name: String,
    pub port: String,
    pub baud_rate: u32,
    pub device_type: DeviceType,
    /// Work-area width in millimetres.
    pub work_area_w: f64,
    /// Work-area height in millimetres.
    pub work_area_h: f64,
}

impl DeviceProfile {
    pub fn new_default() -> Self {
        Self {
            name: "Default".to_owned(),
            port: String::new(),
            baud_rate: 115200,
            device_type: DeviceType::GrblLaser,
            work_area_w: 400.0,
            work_area_h: 400.0,
        }
    }
}

impl std::fmt::Display for DeviceProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Index of the currently-active profile.
    pub active_profile: usize,
    /// All stored profiles (at least one always present).
    pub profiles: Vec<DeviceProfile>,
}

impl DeviceConfig {
    /// Return the active profile (guaranteed to exist).
    pub fn active(&self) -> &DeviceProfile {
        let idx = self.active_profile.min(self.profiles.len().saturating_sub(1));
        &self.profiles[idx]
    }

    /// Return a mutable reference to the active profile.
    pub fn active_mut(&mut self) -> &mut DeviceProfile {
        let idx = self.active_profile.min(self.profiles.len().saturating_sub(1));
        &mut self.profiles[idx]
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            active_profile: 0,
            profiles: vec![DeviceProfile::new_default()],
        }
    }
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
            device: DeviceConfig::default(),
            visual: VisualConfig::default(),
            bindings: KeyBindings::default(),
            mouse_bindings: MouseBindings::default(),
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
            .join("snart-cut")
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
