//! Shared enumerations and lightweight data types used across all modules.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------

/// Supported machine types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeviceType {
    #[default]
    GrblLaser,
    GrblSpindle,
    VinylCutter,
    Marlin,
    RuidaLaser,
    VevorSmart1,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl DeviceType {
    pub fn label(&self) -> &'static str {
        match self {
            DeviceType::GrblLaser => "GRBL Laser",
            DeviceType::GrblSpindle => "GRBL Spindle",
            DeviceType::VinylCutter => "Vinyl Cutter",
            DeviceType::Marlin => "Marlin",
            DeviceType::RuidaLaser  => "Ruida Laser",
            DeviceType::VevorSmart1 => "Vevor Smart 1",
        }
    }

    pub fn all() -> &'static [DeviceType] {
        &[
            DeviceType::GrblLaser,
            DeviceType::GrblSpindle,
            DeviceType::VinylCutter,
            DeviceType::Marlin,
            DeviceType::RuidaLaser,
            DeviceType::VevorSmart1,
        ]
    }
}

/// How a cut layer is processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayerMode {
    /// Vector cut along the path.
    #[default]
    Line,
    /// Raster-fill the enclosed area.
    Fill,
    /// Concentric offset fill.
    OffsetFill,
    /// Grayscale image engraving.
    Image,
}

impl LayerMode {
    pub fn label(&self) -> &'static str {
        match self {
            LayerMode::Line => "Line",
            LayerMode::Fill => "Fill",
            LayerMode::OffsetFill => "Offset Fill",
            LayerMode::Image => "Image",
        }
    }

    pub fn all() -> &'static [LayerMode] {
        &[
            LayerMode::Line,
            LayerMode::Fill,
            LayerMode::OffsetFill,
            LayerMode::Image,
        ]
    }
}

/// Canvas drawing tool identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolType {
    #[default]
    Select,
    Pan,
    Rectangle,
    Ellipse,
    Line,
    Polyline,
    Bezier,
    Text,
    Image,
    Measure,
}

impl ToolType {
    pub fn label(&self) -> &'static str {
        match self {
            ToolType::Select => "Select",
            ToolType::Pan => "Pan",
            ToolType::Rectangle => "Rectangle",
            ToolType::Ellipse => "Ellipse",
            ToolType::Line => "Line",
            ToolType::Polyline => "Polyline",
            ToolType::Bezier => "Bezier",
            ToolType::Text => "Text",
            ToolType::Image => "Image",
            ToolType::Measure => "Measure",
        }
    }

    pub fn all() -> &'static [ToolType] {
        &[
            ToolType::Select,
            ToolType::Pan,
            ToolType::Rectangle,
            ToolType::Ellipse,
            ToolType::Line,
            ToolType::Polyline,
            ToolType::Bezier,
        ]
    }
}

/// Linear measurement units shown in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Units {
    #[default]
    Mm,
    Inch,
}

/// Air-assist state for a laser pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AirAssist {
    Off,
    On,
    #[default]
    Auto,
}

impl AirAssist {
    pub fn label(&self) -> &'static str {
        match self {
            AirAssist::Off => "Off",
            AirAssist::On => "On",
            AirAssist::Auto => "Auto",
        }
    }

    pub fn all() -> &'static [AirAssist] {
        &[AirAssist::Off, AirAssist::On, AirAssist::Auto]
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// 2-D point in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

/// Axis-aligned bounding box in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

// ---------------------------------------------------------------------------
// Path segments
// ---------------------------------------------------------------------------

/// A single segment in a vector path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathSegment {
    MoveTo { x: f64, y: f64 },
    LineTo { x: f64, y: f64 },
    CubicBezierTo { cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64 },
    QuadraticBezierTo { cpx: f64, cpy: f64, x: f64, y: f64 },
    Close,
}

/// A vector path represented as a list of segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PathData {
    pub segments: Vec<PathSegment>,
}

impl PathData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.segments.push(PathSegment::MoveTo { x, y });
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        self.segments.push(PathSegment::LineTo { x, y });
    }

    pub fn cubic_bezier_to(
        &mut self, cp1x: f64, cp1y: f64, cp2x: f64, cp2y: f64, x: f64, y: f64,
    ) {
        self.segments.push(PathSegment::CubicBezierTo { cp1x, cp1y, cp2x, cp2y, x, y });
    }

    pub fn quadratic_bezier_to(&mut self, cpx: f64, cpy: f64, x: f64, y: f64) {
        self.segments.push(PathSegment::QuadraticBezierTo { cpx, cpy, x, y });
    }

    pub fn close(&mut self) {
        self.segments.push(PathSegment::Close);
    }

    /// Add a rectangle.
    pub fn add_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.move_to(x, y);
        self.line_to(x + w, y);
        self.line_to(x + w, y + h);
        self.line_to(x, y + h);
        self.close();
    }

    /// Add an ellipse approximated by four cubic bezier arcs.
    pub fn add_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64) {
        // Kappa constant for circular arc approximation
        const K: f64 = 0.552_284_749;
        let kx = rx * K;
        let ky = ry * K;
        self.move_to(cx + rx, cy);
        self.cubic_bezier_to(cx + rx, cy - ky, cx + kx, cy - ry, cx, cy - ry);
        self.cubic_bezier_to(cx - kx, cy - ry, cx - rx, cy - ky, cx - rx, cy);
        self.cubic_bezier_to(cx - rx, cy + ky, cx - kx, cy + ry, cx, cy + ry);
        self.cubic_bezier_to(cx + kx, cy + ry, cx + rx, cy + ky, cx + rx, cy);
        self.close();
    }

    /// Flatten the path to line segments with the given tolerance (mm).
    pub fn flatten(&self, tolerance: f64) -> Vec<[Point; 2]> {
        let mut lines = Vec::new();
        let mut current = Point::default();
        let mut start = Point::default();

        for seg in &self.segments {
            match seg {
                PathSegment::MoveTo { x, y } => {
                    current = Point::new(*x, *y);
                    start = current;
                }
                PathSegment::LineTo { x, y } => {
                    let end = Point::new(*x, *y);
                    lines.push([current, end]);
                    current = end;
                }
                PathSegment::CubicBezierTo { cp1x, cp1y, cp2x, cp2y, x, y } => {
                    let end = Point::new(*x, *y);
                    flatten_cubic(
                        current,
                        Point::new(*cp1x, *cp1y),
                        Point::new(*cp2x, *cp2y),
                        end,
                        tolerance,
                        &mut lines,
                    );
                    current = end;
                }
                PathSegment::QuadraticBezierTo { cpx, cpy, x, y } => {
                    // Elevate to cubic
                    let cp = Point::new(*cpx, *cpy);
                    let end = Point::new(*x, *y);
                    let cp1 = Point::new(
                        current.x + 2.0 / 3.0 * (cp.x - current.x),
                        current.y + 2.0 / 3.0 * (cp.y - current.y),
                    );
                    let cp2 = Point::new(
                        end.x + 2.0 / 3.0 * (cp.x - end.x),
                        end.y + 2.0 / 3.0 * (cp.y - end.y),
                    );
                    flatten_cubic(current, cp1, cp2, end, tolerance, &mut lines);
                    current = end;
                }
                PathSegment::Close => {
                    if (current.x - start.x).abs() > 1e-9
                        || (current.y - start.y).abs() > 1e-9
                    {
                        lines.push([current, start]);
                    }
                    current = start;
                }
            }
        }
        lines
    }

    /// Compute the approximate bounding box.
    pub fn bounding_box(&self) -> Option<BoundingBox> {
        let pts: Vec<_> = self.segments.iter().flat_map(|s| match s {
            PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => {
                vec![(*x, *y)]
            }
            PathSegment::CubicBezierTo { cp1x, cp1y, cp2x, cp2y, x, y } => {
                vec![(*cp1x, *cp1y), (*cp2x, *cp2y), (*x, *y)]
            }
            PathSegment::QuadraticBezierTo { cpx, cpy, x, y } => {
                vec![(*cpx, *cpy), (*x, *y)]
            }
            PathSegment::Close => vec![],
        }).collect();

        if pts.is_empty() {
            return None;
        }
        let (mut min_x, mut min_y) = pts[0];
        let (mut max_x, mut max_y) = pts[0];
        for (x, y) in &pts[1..] {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
        Some(BoundingBox {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        })
    }
}

/// Recursively flatten a cubic Bézier curve to line segments.
fn flatten_cubic(
    p0: Point, p1: Point, p2: Point, p3: Point,
    tolerance: f64,
    out: &mut Vec<[Point; 2]>,
) {
    // Midpoint subdivision – stop when curve is flat enough.
    let dx = p3.x - p0.x;
    let dy = p3.y - p0.y;
    let d1 = ((p1.x - p0.x) * dy - (p1.y - p0.y) * dx).abs();
    let d2 = ((p2.x - p0.x) * dy - (p2.y - p0.y) * dx).abs();
    let len_sq = dx * dx + dy * dy;
    if (d1 + d2) * (d1 + d2) <= tolerance * tolerance * len_sq {
        out.push([p0, p3]);
        return;
    }
    // Subdivide at t = 0.5
    let m01 = mid(p0, p1);
    let m12 = mid(p1, p2);
    let m23 = mid(p2, p3);
    let m012 = mid(m01, m12);
    let m123 = mid(m12, m23);
    let m0123 = mid(m012, m123);
    flatten_cubic(p0, m01, m012, m0123, tolerance, out);
    flatten_cubic(m0123, m123, m23, p3, tolerance, out);
}

fn mid(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

// ---------------------------------------------------------------------------
// Cut settings
// ---------------------------------------------------------------------------

/// Parameters for a single cut / engrave layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutSettings {
    pub name: String,
    pub mode: LayerMode,
    /// Feed rate in mm/s.
    pub speed_mm_s: f64,
    /// Laser power as a percentage 0–100.
    pub power_pct: f64,
    /// Number of repeated passes.
    pub passes: u32,
    /// Z-axis offset in mm (focus adjustment).
    pub z_offset_mm: f64,
    pub air_assist: AirAssist,
    pub enabled: bool,
    /// Display colour as `#rrggbb` hex string.
    pub color: String,
}

impl Default for CutSettings {
    fn default() -> Self {
        Self {
            name: "Layer".to_owned(),
            mode: LayerMode::Line,
            speed_mm_s: 100.0,
            power_pct: 50.0,
            passes: 1,
            z_offset_mm: 0.0,
            air_assist: AirAssist::Auto,
            enabled: true,
            color: "#ff0000".to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// Workspace configuration
// ---------------------------------------------------------------------------

/// Physical work-area dimensions and display preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub width_mm: f64,
    pub height_mm: f64,
    pub show_grid: bool,
    pub grid_spacing_mm: f64,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            width_mm: 400.0,
            height_mm: 400.0,
            show_grid: true,
            grid_spacing_mm: 10.0,
        }
    }
}
