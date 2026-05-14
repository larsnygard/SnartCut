//! DXF import and export.
//!
//! DXF is a text-based format with group-code / value pairs.  This module
//! implements a minimal parser that handles the entity types most common in
//! laser / vinyl designs:
//!
//! * `LINE`
//! * `CIRCLE`
//! * `ARC`
//! * `LWPOLYLINE`
//! * `POLYLINE` / `VERTEX`
//!
//! All coordinates are in millimetres (DXF R12+ uses drawing units; we
//! assume the drawing unit is already mm).

use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::core::types::PathData;

// ---------------------------------------------------------------------------
// Simple DXF group-code reader
// ---------------------------------------------------------------------------

struct DxfReader<R: BufRead> {
    inner: R,
}

impl<R: BufRead> DxfReader<R> {
    fn new(r: R) -> Self {
        Self { inner: r }
    }

    /// Read the next (group_code, value) pair.
    fn next_pair(&mut self) -> Option<(i32, String)> {
        let mut code_line = String::new();
        let mut val_line = String::new();
        self.inner.read_line(&mut code_line).ok()?;
        self.inner.read_line(&mut val_line).ok()?;
        let code: i32 = code_line.trim().parse().ok()?;
        Some((code, val_line.trim().to_owned()))
    }
}

// ---------------------------------------------------------------------------
// Entity types
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Entity {
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
    Circle { cx: f64, cy: f64, r: f64 },
    Arc { cx: f64, cy: f64, r: f64, start_deg: f64, end_deg: f64 },
    LwPolyline { vertices: Vec<(f64, f64)>, closed: bool },
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/// Load a DXF file and return a list of (PathData, colour) pairs.
/// Colour is always red for now (layer colour mapping is a future enhancement).
pub fn load(path: &Path) -> anyhow::Result<Vec<(PathData, String)>> {
    let file = std::fs::File::open(path)?;
    load_reader(BufReader::new(file))
}

#[allow(unused_assignments)]
fn load_reader<R: BufRead>(reader: R) -> anyhow::Result<Vec<(PathData, String)>> {
    let mut rdr = DxfReader::new(reader);
    let mut in_entities = false;
    let mut result = Vec::new();

    // State for current entity
    let mut entity_type: Option<String> = None;
    let mut x1 = 0f64;
    let mut y1 = 0f64;
    let mut x2 = 0f64;
    let mut y2 = 0f64;
    let mut cx = 0f64;
    let mut cy = 0f64;
    let mut radius = 0f64;
    let mut start_angle = 0f64;
    let mut end_angle = 360f64;
    let mut vertices: Vec<(f64, f64)> = Vec::new();
    let mut poly_flags = 0i32;
    let mut lw_x = 0f64;
    let mut lw_y = 0f64;
    let mut expecting_vertex = false;

    while let Some((code, value)) = rdr.next_pair() {
        match code {
            0 => {
                // Flush previous entity
                if let Some(ref etype) = entity_type.clone() {
                    if let Some(entity) = build_entity(
                        etype,
                        x1, y1, x2, y2,
                        cx, cy, radius,
                        start_angle, end_angle,
                        &vertices,
                        poly_flags,
                    ) {
                        if let Some(pd) = entity_to_path(&entity) {
                            result.push((pd, "#ff0000".to_owned()));
                        }
                    }
                }

                // Reset state
                entity_type = None;
                x1 = 0.0; y1 = 0.0; x2 = 0.0; y2 = 0.0;
                cx = 0.0; cy = 0.0; radius = 0.0;
                start_angle = 0.0; end_angle = 360.0;
                vertices.clear();
                poly_flags = 0;
                lw_x = 0.0; lw_y = 0.0;
                expecting_vertex = false;

                match value.as_str() {
                    "SECTION" => {}
                    "ENDSEC" => {
                        in_entities = false;
                    }
                    "ENTITIES" => {
                        in_entities = true;
                    }
                    "LINE" | "CIRCLE" | "ARC" | "LWPOLYLINE" if in_entities => {
                        entity_type = Some(value.clone());
                    }
                    "VERTEX" if in_entities => {
                        expecting_vertex = true;
                    }
                    _ => {}
                }
            }

            // LINE / ARC / CIRCLE coordinates
            10 => {
                if expecting_vertex {
                    lw_x = value.parse().unwrap_or(0.0);
                } else {
                    x1 = value.parse().unwrap_or(0.0);
                    cx = x1;
                }
            }
            20 => {
                if expecting_vertex {
                    lw_y = value.parse().unwrap_or(0.0);
                    vertices.push((lw_x, lw_y));
                    expecting_vertex = false;
                } else {
                    y1 = value.parse().unwrap_or(0.0);
                    cy = y1;
                }
            }
            11 => x2 = value.parse().unwrap_or(0.0),
            21 => y2 = value.parse().unwrap_or(0.0),
            40 => radius = value.parse().unwrap_or(0.0),
            50 => start_angle = value.parse().unwrap_or(0.0),
            51 => end_angle = value.parse().unwrap_or(360.0),
            // LWPOLYLINE vertex count (70 = flags)
            70 => poly_flags = value.parse().unwrap_or(0),
            _ => {}
        }
    }

    Ok(result)
}

fn build_entity(
    etype: &str,
    x1: f64, y1: f64, x2: f64, y2: f64,
    cx: f64, cy: f64, r: f64,
    start_deg: f64, end_deg: f64,
    vertices: &[(f64, f64)],
    flags: i32,
) -> Option<Entity> {
    match etype {
        "LINE" => Some(Entity::Line { x1, y1, x2, y2 }),
        "CIRCLE" => Some(Entity::Circle { cx, cy, r }),
        "ARC" => Some(Entity::Arc { cx, cy, r, start_deg, end_deg }),
        "LWPOLYLINE" if !vertices.is_empty() => Some(Entity::LwPolyline {
            vertices: vertices.to_vec(),
            closed: flags & 1 != 0,
        }),
        _ => None,
    }
}

fn entity_to_path(entity: &Entity) -> Option<PathData> {
    let mut pd = PathData::new();
    match entity {
        Entity::Line { x1, y1, x2, y2 } => {
            pd.move_to(*x1, *y1);
            pd.line_to(*x2, *y2);
        }
        Entity::Circle { cx, cy, r } => {
            pd.add_ellipse(*cx, *cy, *r, *r);
        }
        Entity::Arc { cx, cy, r, start_deg, end_deg } => {
            arc_to_path(&mut pd, *cx, *cy, *r, *start_deg, *end_deg);
        }
        Entity::LwPolyline { vertices, closed } => {
            if vertices.is_empty() {
                return None;
            }
            pd.move_to(vertices[0].0, vertices[0].1);
            for &(x, y) in &vertices[1..] {
                pd.line_to(x, y);
            }
            if *closed {
                pd.close();
            }
        }
    }
    Some(pd)
}

/// Approximate an arc as cubic Bézier segments.
fn arc_to_path(pd: &mut PathData, cx: f64, cy: f64, r: f64, start_deg: f64, end_deg: f64) {
    let mut span = end_deg - start_deg;
    if span <= 0.0 {
        span += 360.0;
    }

    // Split into ≤90° segments
    let steps = ((span / 90.0).ceil() as usize).max(1);
    let step_deg = span / steps as f64;

    let start_rad = start_deg.to_radians();
    let mut angle = start_rad;

    let x0 = cx + r * angle.cos();
    let y0 = cy + r * angle.sin();
    pd.move_to(x0, y0);

    for _ in 0..steps {
        let end_rad = angle + step_deg.to_radians();
        cubic_arc_segment(pd, cx, cy, r, angle, end_rad);
        angle = end_rad;
    }
}

/// Append a single ≤90° arc as a cubic Bézier.
fn cubic_arc_segment(pd: &mut PathData, cx: f64, cy: f64, r: f64, a0: f64, a1: f64) {
    let k = (4.0 / 3.0) * ((a1 - a0) / 4.0).tan();
    let cos0 = a0.cos();
    let sin0 = a0.sin();
    let cos1 = a1.cos();
    let sin1 = a1.sin();

    let cp1x = cx + r * (cos0 - k * sin0);
    let cp1y = cy + r * (sin0 + k * cos0);
    let cp2x = cx + r * (cos1 + k * sin1);
    let cp2y = cy + r * (sin1 - k * cos1);
    let x = cx + r * cos1;
    let y = cy + r * sin1;

    pd.cubic_bezier_to(cp1x, cp1y, cp2x, cp2y, x, y);
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Write a minimal DXF R2010 file containing the given paths.
pub fn save(
    path: &Path,
    paths: &[(PathData, String)],
) -> anyhow::Result<()> {
    let content = save_string(paths);
    std::fs::write(path, content)?;
    Ok(())
}

pub fn save_string(paths: &[(PathData, String)]) -> String {
    let mut dxf = String::new();

    // Minimal DXF header
    dxf.push_str("  0\nSECTION\n  2\nHEADER\n  0\nENDSEC\n");
    dxf.push_str("  0\nSECTION\n  2\nENTITIES\n");

    for (pd, _color) in paths {
        let lines = pd.flatten(0.05);
        for [a, b] in &lines {
            dxf.push_str(&format!(
                "  0\nLINE\n  8\n0\n\
                 10\n{:.6}\n 20\n{:.6}\n 30\n0.0\n\
                 11\n{:.6}\n 21\n{:.6}\n 31\n0.0\n",
                a.x, a.y, b.x, b.y
            ));
        }
    }

    dxf.push_str("  0\nENDSEC\n  0\nEOF\n");
    dxf
}
