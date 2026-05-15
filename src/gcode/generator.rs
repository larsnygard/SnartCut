//! G-code, HPGL and Ruida `.rd` generator.
//!
//! Converts a list of `(paths, CutSettings)` layer pairs into machine-code
//! command strings.
//!
//! # GRBL (laser mode `$32=1`)
//! * `M3 S<power>` / `M5` – laser on / off
//! * `G0` – rapid move (laser off)
//! * `G1 F<feed>` – cut move (laser on)
//! * `M8` / `M9` – air-assist
//!
//! # HPGL (vinyl cutter)
//! * `IN` – initialise
//! * `SP1` – select blade
//! * `VS<n>` – velocity (cm/s)
//! * `FS<n>` – blade force (g)
//! * `PU<x>,<y>` – pen up + move
//! * `PD<x>,<y>` – pen down + cut
//!
//! # Ruida `.rd`
//! Binary format; output lines are hex-encoded bytes (one per line).
//! The device driver decodes them before transfer.

use crate::core::types::{AirAssist, CutSettings, DeviceType, PathData};
use crate::device::vinyl::mm_to_hpgl;

/// Resolution for flattening bezier curves (mm).
const FLATTEN_TOL: f64 = 0.05;

pub struct GCodeGenerator {
    pub device_type: DeviceType,
    pub workspace_height_mm: f64,
    /// If `true`, invert Y so origin is at bottom-left (LightBurn style).
    pub origin_bottom_left: bool,
    /// Vinyl blade force in grams.
    pub blade_force: u32,
    /// Vinyl cutting speed in cm/s.
    pub cutting_speed: u32,
}

impl Default for GCodeGenerator {
    fn default() -> Self {
        Self {
            device_type: DeviceType::GrblLaser,
            workspace_height_mm: 400.0,
            origin_bottom_left: true,
            blade_force: 80,
            cutting_speed: 10,
        }
    }
}

impl GCodeGenerator {
    pub fn new(device_type: DeviceType, workspace_height_mm: f64) -> Self {
        Self {
            device_type,
            workspace_height_mm,
            ..Default::default()
        }
    }

    /// Generate machine-code lines for the given layers.
    pub fn generate(&self, layers: &[(&[PathData], &CutSettings)]) -> Vec<String> {
        match self.device_type {
            DeviceType::VinylCutter | DeviceType::VevorSmart1 => self.generate_hpgl(layers),
            DeviceType::RuidaLaser  => self.generate_rd(layers),
            _ => self.generate_gcode(layers),
        }
    }

    /// Like [`generate`] but returns a single newline-joined string.
    pub fn generate_string(&self, layers: &[(&[PathData], &CutSettings)]) -> String {
        self.generate(layers).join("\n")
    }

    // ------------------------------------------------------------------
    // G-code
    // ------------------------------------------------------------------

    fn generate_gcode(&self, layers: &[(&[PathData], &CutSettings)]) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        lines.push("; SnartCut generated G-code".to_owned());
        lines.push("G21       ; units mm".to_owned());
        lines.push("G90       ; absolute positioning".to_owned());
        lines.push("G0 X0 Y0  ; home".to_owned());
        lines.push("M5        ; laser off".to_owned());

        for (paths, settings) in layers {
            if !settings.enabled {
                continue;
            }

            let feed = settings.speed_mm_s * 60.0; // mm/s → mm/min
            let power = (settings.power_pct / 100.0 * 1000.0) as u32; // 0–1000

            lines.push(format!("; Layer: {}", settings.name));

            if settings.air_assist == AirAssist::On {
                lines.push("M8  ; air assist on".to_owned());
            }

            for pass in 0..settings.passes {
                if settings.passes > 1 {
                    lines.push(format!("; Pass {}/{}", pass + 1, settings.passes));
                }

                for path in *paths {
                    for segment_lines in self.path_to_gcode(path, feed, power) {
                        lines.extend(segment_lines);
                    }
                }
            }

            if settings.air_assist == AirAssist::On {
                lines.push("M9  ; air assist off".to_owned());
            }
        }

        lines.push("M5        ; laser off".to_owned());
        lines.push("G0 X0 Y0  ; return home".to_owned());

        lines
    }

    fn path_to_gcode(
        &self,
        path: &PathData,
        feed: f64,
        power: u32,
    ) -> Vec<Vec<String>> {
        let flat = path.flatten(FLATTEN_TOL);
        if flat.is_empty() {
            return vec![];
        }

        // Group consecutive segments into sub-paths (split at move_to = lift).
        // We detect a sub-path break when the start of a segment does not equal
        // the end of the previous.
        let mut sub_paths: Vec<Vec<[crate::core::types::Point; 2]>> = Vec::new();
        let mut current_sub: Vec<[crate::core::types::Point; 2]> = Vec::new();

        for seg in &flat {
            if current_sub.is_empty() {
                current_sub.push(*seg);
            } else {
                let last_end = current_sub.last().unwrap()[1];
                if (seg[0].x - last_end.x).abs() < 1e-6
                    && (seg[0].y - last_end.y).abs() < 1e-6
                {
                    current_sub.push(*seg);
                } else {
                    sub_paths.push(std::mem::take(&mut current_sub));
                    current_sub.push(*seg);
                }
            }
        }
        if !current_sub.is_empty() {
            sub_paths.push(current_sub);
        }

        sub_paths
            .into_iter()
            .map(|sub| {
                let mut cmds = Vec::new();
                let start = sub[0][0];
                let sy = self.flip_y(start.y);
                cmds.push(format!("G0 X{:.3} Y{:.3}", start.x, sy));
                cmds.push(format!("M3 S{power}"));
                for seg in &sub {
                    let ey = self.flip_y(seg[1].y);
                    cmds.push(format!(
                        "G1 X{:.3} Y{:.3} F{:.0}",
                        seg[1].x, ey, feed
                    ));
                }
                cmds.push("M5".to_owned());
                cmds
            })
            .collect()
    }

    fn flip_y(&self, y: f64) -> f64 {
        if self.origin_bottom_left {
            self.workspace_height_mm - y
        } else {
            y
        }
    }

    // ------------------------------------------------------------------
    // HPGL
    // ------------------------------------------------------------------

    fn generate_hpgl(&self, layers: &[(&[PathData], &CutSettings)]) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        lines.push("IN;".to_owned());
        lines.push("SP1;".to_owned());
        lines.push(format!("VS{};", self.cutting_speed));
        lines.push(format!("FS{};", self.blade_force));

        for (paths, settings) in layers {
            if !settings.enabled {
                continue;
            }

            for path in *paths {
                let flat = path.flatten(FLATTEN_TOL);
                if flat.is_empty() {
                    continue;
                }

                // Pen up to start
                let start = flat[0][0];
                lines.push(format!(
                    "PU{},{};",
                    mm_to_hpgl(start.x),
                    mm_to_hpgl(start.y)
                ));

                // Pen down along path
                for seg in &flat {
                    let end = seg[1];
                    lines.push(format!(
                        "PD{},{};",
                        mm_to_hpgl(end.x),
                        mm_to_hpgl(end.y)
                    ));
                }
            }
        }

        lines.push("PU0,0;".to_owned());

        lines
    }

    // ------------------------------------------------------------------
    // Ruida `.rd` binary format
    // ------------------------------------------------------------------
    //
    // Coordinates are encoded in µm (1 unit = 1 µm = 0.001 mm) as 5-byte
    // little-endian values split across 5 bytes (the top nibble of each
    // byte encodes which axis and whether sign is negative).
    //
    // For simplicity we use the "absolute coordinate" encoding below which
    // is compatible with all Ruida controllers.  Each byte is XOR-scrambled
    // with 0x88 before transmission; the device driver performs this step.
    //
    // Key command bytes (unscrambled):
    //   0xA8 <x5> <y5>  – rapid move (laser off)
    //   0xA9 <x5> <y5>  – cut move   (laser on)
    //   0xC6 0x01 <byte>               – set min power layer 0  (0–255)
    //   0xC6 0x02 <byte>               – set max power layer 0
    //   0xC6 0x05 <speed3>             – set cut speed (µm/s, 3 bytes big-endian)
    //   0xE7 0x00                      – file header
    //   0xDA 0x01                      – end of file

    fn generate_rd(&self, layers: &[(&[PathData], &CutSettings)]) -> Vec<String> {
        let mut bytes: Vec<u8> = Vec::new();

        // File header
        bytes.extend_from_slice(&[0xD4, 0x00]); // begin job

        for (paths, settings) in layers {
            if !settings.enabled {
                continue;
            }

            let speed_um_s = (settings.speed_mm_s * 1000.0) as u32;
            let power_byte = (settings.power_pct / 100.0 * 255.0).clamp(0.0, 255.0) as u8;

            // Set speed (3 bytes, big-endian, µm/s)
            bytes.push(0xC6);
            bytes.push(0x05);
            bytes.push(((speed_um_s >> 16) & 0xFF) as u8);
            bytes.push(((speed_um_s >> 8)  & 0xFF) as u8);
            bytes.push( (speed_um_s        & 0xFF) as u8);

            // Set min/max power (layer 0)
            bytes.extend_from_slice(&[0xC6, 0x01, power_byte]);
            bytes.extend_from_slice(&[0xC6, 0x02, power_byte]);

            for pass in 0..settings.passes {
                let _ = pass; // passes handled by repeating paths
                for path in *paths {
                    let flat = path.flatten(FLATTEN_TOL);
                    if flat.is_empty() {
                        continue;
                    }

                    // Group into sub-paths
                    let mut subs: Vec<Vec<[crate::core::types::Point; 2]>> = Vec::new();
                    let mut cur: Vec<[crate::core::types::Point; 2]> = Vec::new();
                    for seg in &flat {
                        if cur.is_empty() {
                            cur.push(*seg);
                        } else {
                            let last = cur.last().unwrap()[1];
                            if (seg[0].x - last.x).abs() < 1e-6
                                && (seg[0].y - last.y).abs() < 1e-6
                            {
                                cur.push(*seg);
                            } else {
                                subs.push(std::mem::take(&mut cur));
                                cur.push(*seg);
                            }
                        }
                    }
                    if !cur.is_empty() {
                        subs.push(cur);
                    }

                    for sub in subs {
                        let start = sub[0][0];
                        // Rapid to start (laser off)
                        bytes.push(0xA8);
                        bytes.extend_from_slice(&rd_coord(start.x));
                        bytes.extend_from_slice(&rd_coord(self.flip_y(start.y)));

                        for seg in &sub {
                            let end = seg[1];
                            // Cut move (laser on)
                            bytes.push(0xA9);
                            bytes.extend_from_slice(&rd_coord(end.x));
                            bytes.extend_from_slice(&rd_coord(self.flip_y(end.y)));
                        }
                    }
                }
            }
        }

        // End of file
        bytes.extend_from_slice(&[0xDA, 0x01]);

        // Encode as hex lines so the driver can decode them
        bytes.iter().map(|b| format!("{b:02X}")).collect()
    }
}

/// Encode a coordinate (mm) as Ruida's 5-byte absolute format.
/// Unit: 1 µm = 0.001 mm.  Range: 0 – 1677 mm (20-bit µm value).
fn rd_coord(mm: f64) -> [u8; 5] {
    let um = (mm.max(0.0) * 1000.0).round() as u32;
    // Ruida 5-byte encoding: each byte holds 7 bits of the value in bits 6..0,
    // with bit 7 always 0.  Byte 0 is the most-significant 7 bits.
    [
        ((um >> 28) & 0x7F) as u8,
        ((um >> 21) & 0x7F) as u8,
        ((um >> 14) & 0x7F) as u8,
        ((um >>  7) & 0x7F) as u8,
        ( um        & 0x7F) as u8,
    ]
}
