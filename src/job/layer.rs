//! Cut-layer model.
//!
//! A [`Layer`] pairs a [`CutSettings`] with the set of canvas-item UUIDs
//! that belong to it.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::types::CutSettings;

/// Default layer colours (cycle when adding new layers).
const LAYER_COLORS: &[&str] = &[
    "#e74c3c", "#3498db", "#2ecc71", "#f39c12", "#9b59b6",
    "#1abc9c", "#e67e22", "#34495e", "#e91e63", "#00bcd4",
];

/// A single named cut / engrave layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub settings: CutSettings,
    pub item_ids: HashSet<Uuid>,
}

impl Layer {
    pub fn new(settings: CutSettings) -> Self {
        Self {
            settings,
            item_ids: HashSet::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.settings.name
    }

    pub fn color(&self) -> &str {
        &self.settings.color
    }

    pub fn enabled(&self) -> bool {
        self.settings.enabled
    }

    pub fn add_item(&mut self, id: Uuid) {
        self.item_ids.insert(id);
    }

    pub fn remove_item(&mut self, id: Uuid) {
        self.item_ids.remove(&id);
    }
}

/// Ordered, mutable list of [`Layer`]s.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerList {
    layers: Vec<Layer>,
}

impl LayerList {
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------
    // List interface
    // ------------------------------------------------------------------

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Layer> {
        self.layers.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Layer> {
        self.layers.get_mut(index)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Layer> {
        self.layers.iter()
    }

    // ------------------------------------------------------------------
    // Mutation
    // ------------------------------------------------------------------

    /// Add a new layer with a default colour from the palette.
    pub fn add_new(&mut self) -> usize {
        let color = LAYER_COLORS[self.layers.len() % LAYER_COLORS.len()];
        let mut settings = CutSettings::default();
        settings.name = format!("Layer {}", self.layers.len() + 1);
        settings.color = color.to_owned();
        self.layers.push(Layer::new(settings));
        self.layers.len() - 1
    }

    pub fn add(&mut self, layer: Layer) -> usize {
        self.layers.push(layer);
        self.layers.len() - 1
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.layers.len() {
            self.layers.remove(index);
        }
    }

    pub fn move_up(&mut self, index: usize) {
        if index + 1 < self.layers.len() {
            self.layers.swap(index, index + 1);
        }
    }

    pub fn move_down(&mut self, index: usize) {
        if index > 0 {
            self.layers.swap(index, index - 1);
        }
    }

    /// Return the index of the layer that owns `item_id`, if any.
    pub fn layer_for_item(&self, item_id: Uuid) -> Option<usize> {
        self.layers
            .iter()
            .position(|l| l.item_ids.contains(&item_id))
    }

    /// Assign all selected item IDs to `layer_index`.
    pub fn assign_items(&mut self, layer_index: usize, ids: &[Uuid]) {
        // Remove from any existing layer
        for layer in self.layers.iter_mut() {
            for id in ids {
                layer.item_ids.remove(id);
            }
        }
        // Add to target
        if let Some(layer) = self.layers.get_mut(layer_index) {
            for id in ids {
                layer.item_ids.insert(*id);
            }
        }
    }
}

impl<'a> IntoIterator for &'a LayerList {
    type Item = &'a Layer;
    type IntoIter = std::slice::Iter<'a, Layer>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
