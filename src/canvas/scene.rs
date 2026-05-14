//! Design scene – holds all [`DesignItem`]s and scene-level state.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::canvas::items::DesignItem;
use crate::core::types::PathData;

/// The complete design scene (serialisable for save/load).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scene {
    /// Ordered list of items (bottom → top).
    items: Vec<DesignItem>,
    /// Set of selected item IDs.
    #[serde(skip)]
    selected: Vec<Uuid>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------
    // Item management
    // ------------------------------------------------------------------

    pub fn add_item(&mut self, item: DesignItem) -> Uuid {
        let id = item.id;
        self.items.push(item);
        id
    }

    pub fn add_path(&mut self, path: PathData, color: &str) -> Uuid {
        self.add_item(DesignItem::new(path, color))
    }

    pub fn remove_item(&mut self, id: Uuid) {
        self.items.retain(|i| i.id != id);
        self.selected.retain(|s| *s != id);
    }

    pub fn remove_selected(&mut self) {
        let sel: Vec<_> = self.selected.drain(..).collect();
        for id in sel {
            self.items.retain(|i| i.id != id);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.selected.clear();
    }

    pub fn item(&self, id: Uuid) -> Option<&DesignItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn item_mut(&mut self, id: Uuid) -> Option<&mut DesignItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    pub fn items(&self) -> &[DesignItem] {
        &self.items
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    // ------------------------------------------------------------------
    // Selection
    // ------------------------------------------------------------------

    pub fn selected_ids(&self) -> &[Uuid] {
        &self.selected
    }

    pub fn is_selected(&self, id: Uuid) -> bool {
        self.selected.contains(&id)
    }

    pub fn set_selection(&mut self, ids: Vec<Uuid>) {
        self.selected = ids;
    }

    pub fn select_all(&mut self) {
        self.selected = self.items.iter().map(|i| i.id).collect();
    }

    pub fn deselect_all(&mut self) {
        self.selected.clear();
    }

    /// Hit-test a point and return the topmost item ID, if any.
    pub fn hit_test(&self, x: f64, y: f64, threshold: f64) -> Option<Uuid> {
        self.items.iter().rev().find(|i| i.hit_test(x, y, threshold)).map(|i| i.id)
    }

    /// Return all item IDs whose bounding boxes overlap the given rectangle.
    pub fn items_in_rect(&self, rx: f64, ry: f64, rw: f64, rh: f64) -> Vec<Uuid> {
        self.items
            .iter()
            .filter(|i| {
                if let Some(bb) = i.bounding_box() {
                    bb.x < rx + rw
                        && bb.right() > rx
                        && bb.y < ry + rh
                        && bb.bottom() > ry
                } else {
                    false
                }
            })
            .map(|i| i.id)
            .collect()
    }

    /// Translate all selected items by (dx, dy) mm.
    pub fn translate_selected(&mut self, dx: f64, dy: f64) {
        let sel = self.selected.clone();
        for id in &sel {
            if let Some(item) = self.item_mut(*id) {
                item.translate(dx, dy);
            }
        }
    }

    // ------------------------------------------------------------------
    // Reorder
    // ------------------------------------------------------------------

    pub fn move_item_up(&mut self, id: Uuid) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            if pos + 1 < self.items.len() {
                self.items.swap(pos, pos + 1);
            }
        }
    }

    pub fn move_item_down(&mut self, id: Uuid) {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            if pos > 0 {
                self.items.swap(pos, pos - 1);
            }
        }
    }

    // ------------------------------------------------------------------
    // Serialisation helpers
    // ------------------------------------------------------------------

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}
