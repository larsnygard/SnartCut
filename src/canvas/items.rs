//! Design items – each item wraps a [`PathData`] and has a unique UUID.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::types::{BoundingBox, PathData};

/// Selection handle size in mm.
pub const HANDLE_SIZE: f64 = 3.0;

/// A path-based design item with a unique ID and a layer colour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignItem {
    pub id: Uuid,
    pub path: PathData,
    /// Stroke colour as `#rrggbb`.
    pub color: String,
    /// Translation applied on top of the path coordinates.
    pub translate_x: f64,
    pub translate_y: f64,
}

impl DesignItem {
    pub fn new(path: PathData, color: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            path,
            color: color.into(),
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }

    /// Approximate bounding box (in scene / mm coordinates, after translation).
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        self.path.bounding_box().map(|bb| BoundingBox {
            x: bb.x + self.translate_x,
            y: bb.y + self.translate_y,
            ..bb
        })
    }

    /// Apply a translation delta (called while dragging).
    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.translate_x += dx;
        self.translate_y += dy;
    }

    /// Test whether a point (scene coordinates) is within `threshold` mm of
    /// the bounding box, as a simple hit-test.
    pub fn hit_test(&self, x: f64, y: f64, threshold: f64) -> bool {
        if let Some(bb) = self.bounding_box() {
            x >= bb.x - threshold
                && x <= bb.right() + threshold
                && y >= bb.y - threshold
                && y <= bb.bottom() + threshold
        } else {
            false
        }
    }
}
