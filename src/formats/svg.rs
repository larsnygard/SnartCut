//! SVG import and export using the `usvg` library.
//!
//! All coordinates are converted from SVG user-units to millimetres.
//! `usvg` normalises the SVG tree before we traverse it, so we only see
//! `Path`, `Group`, and `Image` nodes.

use std::path::Path;

use crate::core::types::PathData;

/// Points per inch in SVG (SVG spec: 1 in = 96 px).
const PX_PER_MM: f64 = 96.0 / 25.4;

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/// Load an SVG file and return a list of (PathData, colour) pairs.
pub fn load(path: &Path) -> anyhow::Result<Vec<(PathData, String)>> {
    let data = std::fs::read(path)?;
    load_bytes(&data)
}

/// Parse SVG bytes and return a list of (PathData, colour) pairs.
pub fn load_bytes(data: &[u8]) -> anyhow::Result<Vec<(PathData, String)>> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(data, &opt)?;
    let mut result = Vec::new();
    collect_group(tree.root(), &mut result);
    Ok(result)
}

fn collect_group(group: &usvg::Group, out: &mut Vec<(PathData, String)>) {
    for child in group.children() {
        match child {
            usvg::Node::Group(g) => collect_group(g, out),
            usvg::Node::Path(p) => {
                if let Some(path_data) = convert_path(p) {
                    let color = stroke_color(p);
                    out.push((path_data, color));
                }
            }
            _ => {}
        }
    }
}

fn convert_path(path: &usvg::Path) -> Option<PathData> {
    let transform = path.abs_transform();
    let mut pd = PathData::new();
    let mut has_data = false;

    for seg in path.data().segments() {
        has_data = true;
        match seg {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                let (x, y) = apply_transform(transform, p.x as f64, p.y as f64);
                pd.move_to(x / PX_PER_MM, y / PX_PER_MM);
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                let (x, y) = apply_transform(transform, p.x as f64, p.y as f64);
                pd.line_to(x / PX_PER_MM, y / PX_PER_MM);
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                let (cp1x, cp1y) =
                    apply_transform(transform, p1.x as f64, p1.y as f64);
                let (cp2x, cp2y) =
                    apply_transform(transform, p2.x as f64, p2.y as f64);
                let (x, y) = apply_transform(transform, p3.x as f64, p3.y as f64);
                pd.cubic_bezier_to(
                    cp1x / PX_PER_MM,
                    cp1y / PX_PER_MM,
                    cp2x / PX_PER_MM,
                    cp2y / PX_PER_MM,
                    x / PX_PER_MM,
                    y / PX_PER_MM,
                );
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                let (cpx, cpy) =
                    apply_transform(transform, p1.x as f64, p1.y as f64);
                let (x, y) = apply_transform(transform, p2.x as f64, p2.y as f64);
                pd.quadratic_bezier_to(
                    cpx / PX_PER_MM,
                    cpy / PX_PER_MM,
                    x / PX_PER_MM,
                    y / PX_PER_MM,
                );
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                pd.close();
            }
        }
    }

    if has_data { Some(pd) } else { None }
}

/// Apply a `usvg::Transform` to a point.
fn apply_transform(t: usvg::Transform, x: f64, y: f64) -> (f64, f64) {
    let nx = t.sx as f64 * x + t.kx as f64 * y + t.tx as f64;
    let ny = t.ky as f64 * x + t.sy as f64 * y + t.ty as f64;
    (nx, ny)
}

fn stroke_color(path: &usvg::Path) -> String {
    if let Some(stroke) = path.stroke() {
        if let usvg::Paint::Color(c) = stroke.paint() {
            return format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue);
        }
    }
    "#ff0000".to_owned()
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Serialise a list of (PathData, colour) pairs to an SVG string.
pub fn save_string(
    paths: &[(PathData, String)],
    width_mm: f64,
    height_mm: f64,
) -> String {
    let w_px = width_mm * PX_PER_MM;
    let h_px = height_mm * PX_PER_MM;

    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     width="{w_mm}mm" height="{h_mm}mm"
     viewBox="0 0 {w_px:.3} {h_px:.3}">
"#,
        w_mm = width_mm,
        h_mm = height_mm,
        w_px = w_px,
        h_px = h_px,
    );

    for (path, color) in paths {
        svg.push_str(&format!(
            r#"  <path fill="none" stroke="{color}" stroke-width="0.3" d="{}"/>"#,
            path_to_d(path, PX_PER_MM),
        ));
        svg.push('\n');
    }

    svg.push_str("</svg>\n");
    svg
}

/// Write SVG to a file.
pub fn save(
    path: &Path,
    paths: &[(PathData, String)],
    width_mm: f64,
    height_mm: f64,
) -> anyhow::Result<()> {
    let content = save_string(paths, width_mm, height_mm);
    std::fs::write(path, content)?;
    Ok(())
}

/// Convert a `PathData` to an SVG `d` attribute string.
fn path_to_d(pd: &PathData, scale: f64) -> String {
    use crate::core::types::PathSegment;
    let mut d = String::new();
    for seg in &pd.segments {
        match seg {
            PathSegment::MoveTo { x, y } => {
                d.push_str(&format!("M {:.4} {:.4} ", x * scale, y * scale));
            }
            PathSegment::LineTo { x, y } => {
                d.push_str(&format!("L {:.4} {:.4} ", x * scale, y * scale));
            }
            PathSegment::CubicBezierTo { cp1x, cp1y, cp2x, cp2y, x, y } => {
                d.push_str(&format!(
                    "C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} ",
                    cp1x * scale,
                    cp1y * scale,
                    cp2x * scale,
                    cp2y * scale,
                    x * scale,
                    y * scale,
                ));
            }
            PathSegment::QuadraticBezierTo { cpx, cpy, x, y } => {
                d.push_str(&format!(
                    "Q {:.4} {:.4} {:.4} {:.4} ",
                    cpx * scale,
                    cpy * scale,
                    x * scale,
                    y * scale,
                ));
            }
            PathSegment::Close => d.push_str("Z "),
        }
    }
    d.trim_end().to_owned()
}
