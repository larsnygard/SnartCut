//! Interactive drawing tools.

use crate::core::types::{PathData, ToolType};

/// State carried by whatever drawing tool is currently active.
#[derive(Debug, Clone, Default)]
pub enum ToolState {
    /// Nothing in progress.
    #[default]
    Idle,
    /// Drawing a rectangle: start corner (mm).
    DrawingRect { start_x: f64, start_y: f64, cur_x: f64, cur_y: f64 },
    /// Drawing an ellipse: bounding-box start corner.
    DrawingEllipse { start_x: f64, start_y: f64, cur_x: f64, cur_y: f64 },
    /// Drawing a straight line.
    DrawingLine { start_x: f64, start_y: f64, cur_x: f64, cur_y: f64 },
    /// Drawing a polyline: accumulated vertices plus live tip.
    DrawingPolyline { points: Vec<(f64, f64)>, cur_x: f64, cur_y: f64 },
    /// Dragging selected items.
    Dragging { last_x: f64, last_y: f64 },
    /// Rubber-band selection.
    Selecting { start_x: f64, start_y: f64, cur_x: f64, cur_y: f64 },
    /// Panning: last cursor position.
    Panning { last_x: f64, last_y: f64 },
}

impl ToolState {
    /// Build a preview `PathData` to show while a shape is being drawn.
    pub fn preview_path(&self) -> Option<PathData> {
        match self {
            ToolState::DrawingRect { start_x, start_y, cur_x, cur_y } => {
                let x = start_x.min(*cur_x);
                let y = start_y.min(*cur_y);
                let w = (cur_x - start_x).abs();
                let h = (cur_y - start_y).abs();
                if w < 0.1 || h < 0.1 {
                    return None;
                }
                let mut p = PathData::new();
                p.add_rect(x, y, w, h);
                Some(p)
            }
            ToolState::DrawingEllipse { start_x, start_y, cur_x, cur_y } => {
                let w = (cur_x - start_x).abs();
                let h = (cur_y - start_y).abs();
                if w < 0.1 || h < 0.1 {
                    return None;
                }
                let cx = (start_x + cur_x) / 2.0;
                let cy = (start_y + cur_y) / 2.0;
                let mut p = PathData::new();
                p.add_ellipse(cx, cy, w / 2.0, h / 2.0);
                Some(p)
            }
            ToolState::DrawingLine { start_x, start_y, cur_x, cur_y } => {
                let mut p = PathData::new();
                p.move_to(*start_x, *start_y);
                p.line_to(*cur_x, *cur_y);
                Some(p)
            }
            ToolState::DrawingPolyline { points, cur_x, cur_y } => {
                if points.is_empty() {
                    return None;
                }
                let mut p = PathData::new();
                p.move_to(points[0].0, points[0].1);
                for &(x, y) in &points[1..] {
                    p.line_to(x, y);
                }
                p.line_to(*cur_x, *cur_y);
                Some(p)
            }
            ToolState::Selecting { start_x, start_y, cur_x, cur_y } => {
                let x = start_x.min(*cur_x);
                let y = start_y.min(*cur_y);
                let w = (cur_x - start_x).abs();
                let h = (cur_y - start_y).abs();
                let mut p = PathData::new();
                p.add_rect(x, y, w, h);
                Some(p)
            }
            _ => None,
        }
    }

    /// Finish drawing and return the completed `PathData`, if any.
    pub fn finish_path(&self, tool: ToolType) -> Option<PathData> {
        match (tool, self) {
            (ToolType::Rectangle, ToolState::DrawingRect { start_x, start_y, cur_x, cur_y }) => {
                let x = start_x.min(*cur_x);
                let y = start_y.min(*cur_y);
                let w = (cur_x - start_x).abs();
                let h = (cur_y - start_y).abs();
                if w < 0.1 || h < 0.1 {
                    return None;
                }
                let mut p = PathData::new();
                p.add_rect(x, y, w, h);
                Some(p)
            }
            (ToolType::Ellipse, ToolState::DrawingEllipse { start_x, start_y, cur_x, cur_y }) => {
                let w = (cur_x - start_x).abs();
                let h = (cur_y - start_y).abs();
                if w < 0.1 || h < 0.1 {
                    return None;
                }
                let cx = (start_x + cur_x) / 2.0;
                let cy = (start_y + cur_y) / 2.0;
                let mut p = PathData::new();
                p.add_ellipse(cx, cy, w / 2.0, h / 2.0);
                Some(p)
            }
            (ToolType::Line, ToolState::DrawingLine { start_x, start_y, cur_x, cur_y }) => {
                let dx = cur_x - start_x;
                let dy = cur_y - start_y;
                if dx * dx + dy * dy < 0.01 {
                    return None;
                }
                let mut p = PathData::new();
                p.move_to(*start_x, *start_y);
                p.line_to(*cur_x, *cur_y);
                Some(p)
            }
            (ToolType::Polyline, ToolState::DrawingPolyline { points, .. }) => {
                if points.len() < 2 {
                    return None;
                }
                let mut p = PathData::new();
                p.move_to(points[0].0, points[0].1);
                for &(x, y) in &points[1..] {
                    p.line_to(x, y);
                }
                Some(p)
            }
            _ => None,
        }
    }
}
