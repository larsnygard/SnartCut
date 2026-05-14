//! Top-level job settings and built-in material library.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::types::{AirAssist, CutSettings, LayerMode, WorkspaceConfig};
use crate::job::layer::{Layer, LayerList};

// ---------------------------------------------------------------------------
// Material presets
// ---------------------------------------------------------------------------

/// Built-in material preset library.
pub fn material_library() -> HashMap<&'static str, CutSettings> {
    let mut m = HashMap::new();

    m.insert(
        "Plywood 3mm (cut)",
        CutSettings {
            name: "Plywood 3mm cut".to_owned(),
            mode: LayerMode::Line,
            speed_mm_s: 30.0,
            power_pct: 90.0,
            passes: 3,
            air_assist: AirAssist::On,
            color: "#e74c3c".to_owned(),
            ..Default::default()
        },
    );
    m.insert(
        "Plywood 3mm (engrave)",
        CutSettings {
            name: "Plywood 3mm engrave".to_owned(),
            mode: LayerMode::Fill,
            speed_mm_s: 200.0,
            power_pct: 60.0,
            passes: 1,
            air_assist: AirAssist::Auto,
            color: "#3498db".to_owned(),
            ..Default::default()
        },
    );
    m.insert(
        "Acrylic 3mm (cut)",
        CutSettings {
            name: "Acrylic 3mm cut".to_owned(),
            mode: LayerMode::Line,
            speed_mm_s: 15.0,
            power_pct: 95.0,
            passes: 2,
            air_assist: AirAssist::On,
            color: "#9b59b6".to_owned(),
            ..Default::default()
        },
    );
    m.insert(
        "Cardboard (cut)",
        CutSettings {
            name: "Cardboard cut".to_owned(),
            mode: LayerMode::Line,
            speed_mm_s: 80.0,
            power_pct: 40.0,
            passes: 1,
            air_assist: AirAssist::Auto,
            color: "#e67e22".to_owned(),
            ..Default::default()
        },
    );
    m.insert(
        "Leather (engrave)",
        CutSettings {
            name: "Leather engrave".to_owned(),
            mode: LayerMode::Fill,
            speed_mm_s: 150.0,
            power_pct: 35.0,
            passes: 1,
            air_assist: AirAssist::Off,
            color: "#795548".to_owned(),
            ..Default::default()
        },
    );
    m.insert(
        "Vinyl (cut)",
        CutSettings {
            name: "Vinyl cut".to_owned(),
            mode: LayerMode::Line,
            speed_mm_s: 50.0,
            power_pct: 0.0,
            passes: 1,
            air_assist: AirAssist::Off,
            color: "#000000".to_owned(),
            ..Default::default()
        },
    );

    m
}

// ---------------------------------------------------------------------------
// JobSettings
// ---------------------------------------------------------------------------

/// Complete description of a cutting / engraving job.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobSettings {
    pub workspace: WorkspaceConfig,
    pub layers: LayerList,
    pub material: String,
    pub notes: String,
}

impl JobSettings {
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------
    // Serialisation
    // ------------------------------------------------------------------

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json(&text)
    }
}
