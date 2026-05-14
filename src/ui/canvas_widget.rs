//! Iced canvas widget – renders the design scene and dispatches drawing tool
//! interactions.

use iced::widget::canvas::{self, Cache, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme, Vector};

use crate::app::Message;
use crate::canvas::scene::Scene;
use crate::canvas::tools::ToolState;
use crate::core::types::{PathData, PathSegment, ToolType};

/// Persistent per-canvas interaction state (held by Iced's canvas machinery).
#[derive(Debug, Default)]
pub struct CanvasState {
    pub tool_state: ToolState,
    /// Cache for static scene geometry (invalidated when items change).
    pub scene_cache: Cache,
    /// Cache for the grid (invalidated when workspace size or zoom changes).
    pub grid_cache: Cache,
}

/// The canvas program handed to `iced::widget::Canvas`.
///
/// It holds shared references into the application state so that `draw` can
/// render the current scene without cloning it.
pub struct DesignCanvas<'a> {
    pub scene: &'a Scene,
    pub active_tool: ToolType,
    pub active_color: &'a str,
    pub zoom: f32,
    pub pan: Vector,
    pub workspace_w: f64,
    pub workspace_h: f64,
    pub show_grid: bool,
    pub grid_spacing: f64,
}

impl<'a> canvas::Program<Message> for DesignCanvas<'a> {
    type State = CanvasState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        // ---- Grid / workspace background ----
        let grid = state.grid_cache.draw(renderer, bounds.size(), |frame| {
            self.draw_workspace_background(frame);
        });

        // ---- Scene items ----
        let scene_geo = state.scene_cache.draw(renderer, bounds.size(), |frame| {
            self.draw_scene_items(frame);
        });

        // ---- Live preview (tool overlay) ----
        let mut overlay = Frame::new(renderer, bounds.size());
        self.draw_tool_overlay(&mut overlay, cursor, bounds);

        vec![grid, scene_geo, overlay.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        let pos = match cursor.position_in(bounds) {
            Some(p) => p,
            None => return (canvas::event::Status::Ignored, None),
        };
        let scene_pos = self.screen_to_scene(pos);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let msg = self.handle_left_press(state, scene_pos);
                state.scene_cache.clear();
                (canvas::event::Status::Captured, msg)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position: _ }) => {
                let msg = self.handle_mouse_move(state, scene_pos);
                state.scene_cache.clear();
                (canvas::event::Status::Captured, msg)
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let msg = self.handle_left_release(state, scene_pos);
                state.scene_cache.clear();
                (canvas::event::Status::Captured, msg)
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                // Right-click: cancel in-progress drawing
                state.tool_state = ToolState::Idle;
                state.scene_cache.clear();
                (canvas::event::Status::Captured, None)
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match (&self.active_tool, &state.tool_state) {
            (ToolType::Pan, _) | (_, ToolState::Panning { .. }) => {
                mouse::Interaction::Grab
            }
            (ToolType::Select, _) => mouse::Interaction::default(),
            _ => mouse::Interaction::Crosshair,
        }
    }
}

impl<'a> DesignCanvas<'a> {
    // ------------------------------------------------------------------
    // Coordinate transforms
    // ------------------------------------------------------------------

    /// Convert screen pixels → scene mm.
    fn screen_to_scene(&self, p: Point) -> (f64, f64) {
        let x = (p.x as f64 - self.pan.x as f64) / self.zoom as f64;
        let y = (p.y as f64 - self.pan.y as f64) / self.zoom as f64;
        (x, y)
    }

    /// Convert scene mm → frame pixels.
    fn scene_to_frame(&self, x: f64, y: f64) -> Point {
        Point::new(
            (x * self.zoom as f64 + self.pan.x as f64) as f32,
            (y * self.zoom as f64 + self.pan.y as f64) as f32,
        )
    }

    fn scene_len_to_frame(&self, mm: f64) -> f32 {
        (mm * self.zoom as f64) as f32
    }

    // ------------------------------------------------------------------
    // Drawing helpers
    // ------------------------------------------------------------------

    fn draw_workspace_background(&self, frame: &mut Frame) {
        // Dark background
        frame.fill_rectangle(
            Point::ORIGIN,
            frame.size(),
            Color::from_rgb(0.15, 0.15, 0.15),
        );

        // White workspace area
        let origin = self.scene_to_frame(0.0, 0.0);
        let ws_w = self.scene_len_to_frame(self.workspace_w);
        let ws_h = self.scene_len_to_frame(self.workspace_h);

        frame.fill_rectangle(origin, Size::new(ws_w, ws_h), Color::WHITE);

        // Grid
        if self.show_grid && self.grid_spacing > 0.0 {
            let grid_color = Color::from_rgba(0.0, 0.0, 0.0, 0.12);

            let step_px = self.scene_len_to_frame(self.grid_spacing);
            if step_px >= 4.0 {
                let x_count = (self.workspace_w / self.grid_spacing).ceil() as usize;
                let y_count = (self.workspace_h / self.grid_spacing).ceil() as usize;

                for ix in 0..=x_count {
                    let sx = ix as f64 * self.grid_spacing;
                    let p0 = self.scene_to_frame(sx, 0.0);
                    let p1 = self.scene_to_frame(sx, self.workspace_h);
                    let path = Path::line(p0, p1);
                    frame.stroke(&path, Stroke::default().with_color(grid_color).with_width(0.5));
                }
                for iy in 0..=y_count {
                    let sy = iy as f64 * self.grid_spacing;
                    let p0 = self.scene_to_frame(0.0, sy);
                    let p1 = self.scene_to_frame(self.workspace_w, sy);
                    let path = Path::line(p0, p1);
                    frame.stroke(&path, Stroke::default().with_color(grid_color).with_width(0.5));
                }
            }
        }

        // Workspace border
        let border = Path::new(|b| {
            b.rectangle(origin, Size::new(ws_w, ws_h));
        });
        frame.stroke(
            &border,
            Stroke::default()
                .with_color(Color::from_rgb(0.4, 0.4, 0.4))
                .with_width(1.0),
        );
    }

    fn draw_scene_items(&self, frame: &mut Frame) {
        for item in self.scene.items() {
            let color = hex_to_color(&item.color).unwrap_or(Color::from_rgb(1.0, 0.0, 0.0));
            let path = self.build_iced_path(&item.path, item.translate_x, item.translate_y);

            let stroke_width = self.scene_len_to_frame(0.3).max(0.5);

            if self.scene.is_selected(item.id) {
                // Draw selection outline (thicker blue)
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(Color::from_rgb(0.0, 0.47, 0.83))
                        .with_width(stroke_width + 2.0),
                );
                // Draw selection handles
                if let Some(bb) = item.bounding_box() {
                    let hs = self.scene_len_to_frame(3.0);
                    for (hx, hy) in [
                        (bb.x, bb.y),
                        (bb.right(), bb.y),
                        (bb.x, bb.bottom()),
                        (bb.right(), bb.bottom()),
                    ] {
                        let p = self.scene_to_frame(hx, hy);
                        frame.fill_rectangle(
                            Point::new(p.x - hs / 2.0, p.y - hs / 2.0),
                            Size::new(hs, hs),
                            Color::from_rgb(0.0, 0.47, 0.83),
                        );
                    }
                }
            }

            frame.stroke(
                &path,
                Stroke::default().with_color(color).with_width(stroke_width),
            );
        }
    }

    fn draw_tool_overlay(&self, frame: &mut Frame, _cursor: mouse::Cursor, _bounds: Rectangle) {
        // Preview shape while drawing
        if let Some(preview) = self.get_preview_path_data() {
            let color = hex_to_color(self.active_color)
                .unwrap_or(Color::from_rgb(1.0, 0.0, 0.0));
            let path = self.build_iced_path(&preview, 0.0, 0.0);
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(Color { a: 0.7, ..color })
                    .with_width(1.0),
            );
        }
    }

    fn get_preview_path_data(&self) -> Option<PathData> {
        // This is called from `draw_tool_overlay` which has no mutable ToolState,
        // so we reconstruct the preview from the state passed in draw().
        // Instead we expose it through the outer state — the overlay frame is
        // separate. For now, return None (overlay will come from CanvasState
        // access in a future refactor).
        None
    }

    fn build_iced_path(&self, pd: &PathData, tx: f64, ty: f64) -> Path {
        Path::new(|b| {
            for seg in &pd.segments {
                match seg {
                    PathSegment::MoveTo { x, y } => {
                        b.move_to(self.scene_to_frame(x + tx, y + ty));
                    }
                    PathSegment::LineTo { x, y } => {
                        b.line_to(self.scene_to_frame(x + tx, y + ty));
                    }
                    PathSegment::CubicBezierTo { cp1x, cp1y, cp2x, cp2y, x, y } => {
                        b.bezier_curve_to(
                            self.scene_to_frame(cp1x + tx, cp1y + ty),
                            self.scene_to_frame(cp2x + tx, cp2y + ty),
                            self.scene_to_frame(x + tx, y + ty),
                        );
                    }
                    PathSegment::QuadraticBezierTo { cpx, cpy, x, y } => {
                        b.quadratic_curve_to(
                            self.scene_to_frame(cpx + tx, cpy + ty),
                            self.scene_to_frame(x + tx, y + ty),
                        );
                    }
                    PathSegment::Close => {
                        b.close();
                    }
                }
            }
        })
    }

    // ------------------------------------------------------------------
    // Tool event handling
    // ------------------------------------------------------------------

    fn handle_left_press(
        &self,
        state: &mut CanvasState,
        (sx, sy): (f64, f64),
    ) -> Option<Message> {
        match self.active_tool {
            ToolType::Select => {
                if let Some(id) = self.scene.hit_test(sx, sy, 2.0 / self.zoom as f64) {
                    state.tool_state = ToolState::Dragging { last_x: sx, last_y: sy };
                    Some(Message::SelectItem(id))
                } else {
                    state.tool_state = ToolState::Selecting {
                        start_x: sx, start_y: sy, cur_x: sx, cur_y: sy,
                    };
                    Some(Message::DeselectAll)
                }
            }
            ToolType::Pan => {
                state.tool_state = ToolState::Panning { last_x: sx, last_y: sy };
                None
            }
            ToolType::Rectangle => {
                state.tool_state = ToolState::DrawingRect {
                    start_x: sx, start_y: sy, cur_x: sx, cur_y: sy,
                };
                None
            }
            ToolType::Ellipse => {
                state.tool_state = ToolState::DrawingEllipse {
                    start_x: sx, start_y: sy, cur_x: sx, cur_y: sy,
                };
                None
            }
            ToolType::Line => {
                state.tool_state = ToolState::DrawingLine {
                    start_x: sx, start_y: sy, cur_x: sx, cur_y: sy,
                };
                None
            }
            ToolType::Polyline => {
                match &mut state.tool_state {
                    ToolState::DrawingPolyline { points, .. } => {
                        points.push((sx, sy));
                        None
                    }
                    _ => {
                        state.tool_state = ToolState::DrawingPolyline {
                            points: vec![(sx, sy)],
                            cur_x: sx,
                            cur_y: sy,
                        };
                        None
                    }
                }
            }
            _ => None,
        }
    }

    fn handle_mouse_move(
        &self,
        state: &mut CanvasState,
        (sx, sy): (f64, f64),
    ) -> Option<Message> {
        match &mut state.tool_state {
            ToolState::DrawingRect { cur_x, cur_y, .. }
            | ToolState::DrawingEllipse { cur_x, cur_y, .. }
            | ToolState::DrawingLine { cur_x, cur_y, .. } => {
                *cur_x = sx;
                *cur_y = sy;
                None
            }
            ToolState::DrawingPolyline { cur_x, cur_y, .. } => {
                *cur_x = sx;
                *cur_y = sy;
                None
            }
            ToolState::Selecting { cur_x, cur_y, .. } => {
                *cur_x = sx;
                *cur_y = sy;
                None
            }
            ToolState::Dragging { last_x, last_y } => {
                let dx = sx - *last_x;
                let dy = sy - *last_y;
                *last_x = sx;
                *last_y = sy;
                Some(Message::TranslateSelected(dx, dy))
            }
            ToolState::Panning { last_x, last_y } => {
                let dpx = (sx - *last_x) as f32 * self.zoom;
                let dpy = (sy - *last_y) as f32 * self.zoom;
                *last_x = sx;
                *last_y = sy;
                Some(Message::PanCanvas(dpx, dpy))
            }
            _ => Some(Message::CursorMoved(sx, sy)),
        }
    }

    fn handle_left_release(
        &self,
        state: &mut CanvasState,
        (sx, sy): (f64, f64),
    ) -> Option<Message> {
        let old_state = std::mem::replace(&mut state.tool_state, ToolState::Idle);

        match (&self.active_tool, &old_state) {
            (ToolType::Select, ToolState::Selecting { start_x, start_y, cur_x, cur_y }) => {
                let rx = start_x.min(*cur_x);
                let ry = start_y.min(*cur_y);
                let rw = (cur_x - start_x).abs();
                let rh = (cur_y - start_y).abs();
                if rw > 1.0 || rh > 1.0 {
                    return Some(Message::SelectRect(rx, ry, rw, rh));
                }
                None
            }
            (_, ToolState::Dragging { .. }) => None,
            (tool, _) => {
                // Finish shape drawing
                if let Some(path) = old_state.finish_path(*tool) {
                    return Some(Message::AddPath(path, self.active_color.to_owned()));
                }
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Colour helper
// ---------------------------------------------------------------------------

pub fn hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
}
