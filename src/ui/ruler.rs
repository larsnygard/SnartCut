//! Horizontal and vertical mm rulers that track the canvas pan/zoom.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{
    mouse, Color, Element, Font, Length, Pixels, Point, Rectangle, Renderer,
    Size, Theme, Vector,
};

pub const RULER_THICKNESS: f32 = 20.0;

// Tick colours
const BG: Color = Color {
    r: 0.14,
    g: 0.14,
    b: 0.14,
    a: 1.0,
};
const TICK_COLOR: Color = Color {
    r: 0.55,
    g: 0.55,
    b: 0.55,
    a: 1.0,
};
const LABEL_COLOR: Color = Color {
    r: 0.70,
    g: 0.70,
    b: 0.70,
    a: 1.0,
};

// ---------------------------------------------------------------------------
// Shared ruler state
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct RulerState {
    cache: canvas::Cache,
}

impl RulerState {
    pub fn invalidate(&mut self) {
        self.cache.clear();
    }
}

// ---------------------------------------------------------------------------
// Horizontal ruler
// ---------------------------------------------------------------------------

pub struct HRuler {
    pub zoom: f32,
    pub pan_x: f32,
}

impl canvas::Program<crate::app::Message> for HRuler {
    type State = RulerState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let geo = state.cache.draw(renderer, bounds.size(), |frame| {
            draw_h_ruler(frame, bounds.size(), self.zoom, self.pan_x);
        });
        vec![geo]
    }
}

fn draw_h_ruler(frame: &mut Frame, size: Size, zoom: f32, pan_x: f32) {
    // Background
    frame.fill_rectangle(Point::ORIGIN, size, BG);

    // Tick spacing: choose a step in mm that gives reasonable pixel spacing.
    let step_mm = nice_step(10.0 / zoom);
    let _step_px = step_mm * zoom;

    // First tick position in scene mm (left edge of ruler in scene coords)
    let origin_mm = -pan_x / zoom;
    let first_tick = (origin_mm / step_mm).ceil() * step_mm;

    let mut mm = first_tick;
    while mm * zoom + pan_x < size.width {
        let px = mm * zoom + pan_x;

        let is_major = is_major_tick(mm, step_mm);
        let tick_h = if is_major {
            size.height * 0.55
        } else {
            size.height * 0.3
        };

        // Tick line
        let path = Path::line(
            Point::new(px, size.height),
            Point::new(px, size.height - tick_h),
        );
        frame.stroke(
            &path,
            Stroke::default().with_color(TICK_COLOR).with_width(1.0),
        );

        // Label on major ticks
        if is_major {
            let label = format_mm(mm);
            frame.fill_text(Text {
                content: label,
                position: Point::new(px + 2.0, 1.0),
                color: LABEL_COLOR,
                size: Pixels(9.0),
                font: Font::MONOSPACE,
                ..Text::default()
            });
        }

        mm += step_mm;
    }

    // Bottom border line
    let border = Path::line(
        Point::new(0.0, size.height - 0.5),
        Point::new(size.width, size.height - 0.5),
    );
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(Color::from_rgb(0.28, 0.28, 0.28))
            .with_width(1.0),
    );
}

// ---------------------------------------------------------------------------
// Vertical ruler
// ---------------------------------------------------------------------------

pub struct VRuler {
    pub zoom: f32,
    pub pan_y: f32,
}

impl canvas::Program<crate::app::Message> for VRuler {
    type State = RulerState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let geo = state.cache.draw(renderer, bounds.size(), |frame| {
            draw_v_ruler(frame, bounds.size(), self.zoom, self.pan_y);
        });
        vec![geo]
    }
}

fn draw_v_ruler(frame: &mut Frame, size: Size, zoom: f32, pan_y: f32) {
    // Background
    frame.fill_rectangle(Point::ORIGIN, size, BG);

    let step_mm = nice_step(10.0 / zoom);
    let _step_px = step_mm * zoom;

    let origin_mm = -pan_y / zoom;
    let first_tick = (origin_mm / step_mm).ceil() * step_mm;

    let mut mm = first_tick;
    while mm * zoom + pan_y < size.height {
        let py = mm * zoom + pan_y;

        let is_major = is_major_tick(mm, step_mm);
        let tick_w = if is_major {
            size.width * 0.55
        } else {
            size.width * 0.3
        };

        let path = Path::line(
            Point::new(size.width, py),
            Point::new(size.width - tick_w, py),
        );
        frame.stroke(
            &path,
            Stroke::default().with_color(TICK_COLOR).with_width(1.0),
        );

        if is_major {
            let label = format_mm(mm);
            // Draw label rotated 90° using a transform
            frame.with_save(|f| {
                f.translate(Vector::new(size.width - tick_w - 1.0, py - 1.0));
                f.rotate(-std::f32::consts::FRAC_PI_2);
                f.fill_text(Text {
                    content: label,
                    position: Point::ORIGIN,
                    color: LABEL_COLOR,
                    size: Pixels(9.0),
                    font: Font::MONOSPACE,
                    ..Text::default()
                });
            });
        }

        mm += step_mm;
    }

    // Right border line
    let border = Path::line(
        Point::new(size.width - 0.5, 0.0),
        Point::new(size.width - 0.5, size.height),
    );
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(Color::from_rgb(0.28, 0.28, 0.28))
            .with_width(1.0),
    );
}

// ---------------------------------------------------------------------------
// Convenience view functions called from app.rs
// ---------------------------------------------------------------------------

pub fn h_ruler(zoom: f32, pan_x: f32) -> iced::widget::Canvas<HRuler, crate::app::Message> {
    iced::widget::canvas(HRuler { zoom, pan_x })
        .width(Length::Fill)
        .height(Length::Fixed(RULER_THICKNESS))
}

pub fn v_ruler(zoom: f32, pan_y: f32) -> iced::widget::Canvas<VRuler, crate::app::Message> {
    iced::widget::canvas(VRuler { zoom, pan_y })
        .width(Length::Fixed(RULER_THICKNESS))
        .height(Length::Fill)
}

// ---------------------------------------------------------------------------
// Corner square (top-left, where rulers meet)
// ---------------------------------------------------------------------------

pub fn corner<'a>() -> Element<'a, crate::app::Message> {
    iced::widget::container(iced::widget::Space::new(
        Length::Fixed(RULER_THICKNESS),
        Length::Fixed(RULER_THICKNESS),
    ))
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(BG)),
        text_color: None,
        border: iced::Border {
            color: Color::from_rgb(0.28, 0.28, 0.28),
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Default::default(),
    })
    .into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Choose a tick step that keeps pixel spacing comfortable.
/// Returns step size in mm.
fn nice_step(mm_per_10px: f32) -> f32 {
    // Candidate steps in mm
    const STEPS: &[f32] = &[1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0];
    for &s in STEPS {
        if s >= mm_per_10px {
            return s;
        }
    }
    500.0
}

/// Major tick every 5 steps (e.g. labelled tick every 50 mm when step is 10 mm).
fn is_major_tick(mm: f32, step_mm: f32) -> bool {
    let v = (mm / step_mm).round() as i64;
    v % 5 == 0
}

/// Format a mm value for display on the ruler.
fn format_mm(mm: f32) -> String {
    if mm.fract() == 0.0 {
        format!("{}", mm as i64)
    } else {
        format!("{:.1}", mm)
    }
}
