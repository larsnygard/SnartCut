//! Vevor Smart1 USB Protocol – reverse-engineered from packet captures.
//!
//! The Vevor Smart1 uses a binary protocol over USB bulk transfers.
//! Each message consists of a 28-byte header followed by payload data.
//!
//! **Header structure (bytes 0-27):**
//! - Byte 0: 0x1B (ESC marker)
//! - Byte 1: 0x00 (padding)
//! - Bytes 2-5: CRC32 or checksum (calculated from bytes 6-27 + payload)
//! - Bytes 6-23: Fixed constant bytes (0991ffff000000000900000300180002)
//! - Bytes 24-25: Payload length (little-endian u16)
//! - Bytes 26-27: 0x0000 (padding or part of checksum)
//!
//! **Payload:**
//! - ASCII HPGL commands or device-specific commands.
//! - Examples: IN;, SP1;, FS10;, VS250;, PU..., PD...
//!            setmat:0;, TB42;, TB42:LOADSUCCESS;
//!
//! **Known fixed bytes:**
//! - 0991ffff000000000900000300180002 (bytes 6-23)

use crc32fast::Hasher;

/// State machine for Vevor job workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VevorJobState {
    Idle,
    AwaitingMediaLoad,
    MediaLoaded,
    AwaitingStart,
    Cutting,
    CutFinished,
    Error,
}

impl VevorJobState {
    pub fn as_str(&self) -> &'static str {
        match self {
            VevorJobState::Idle => "Idle",
            VevorJobState::AwaitingMediaLoad => "Awaiting Media Load",
            VevorJobState::MediaLoaded => "Media Loaded",
            VevorJobState::AwaitingStart => "Awaiting Start",
            VevorJobState::Cutting => "Cutting",
            VevorJobState::CutFinished => "Cut Finished",
            VevorJobState::Error => "Error",
        }
    }
}

/// Generate a Vevor USB protocol message with HPGL payload.
pub fn generate_hpgl_message(hpgl_commands: &str) -> Vec<u8> {
    let payload = hpgl_commands.as_bytes();
    generate_message(payload)
}

/// Generate "prepare for loading" command.
pub fn generate_prepare_job_command() -> Vec<u8> {
    generate_message(b"setmat:0;")
}

/// Generate "prepare for loading" command with explicit material and sheet size.
///
/// Observed in captures as: setmat:1;JS12192,12192;
pub fn generate_prepare_job_with_size(material: u8, width_units: u32, height_units: u32) -> Vec<u8> {
    let payload = format!("setmat:{material};JS{width_units},{height_units};");
    generate_message(payload.as_bytes())
}

/// Generate status poll command.
pub fn generate_status_poll_command() -> Vec<u8> {
    generate_message(b"TB42;")
}

/// Generate eject media command.
pub fn generate_eject_command() -> Vec<u8> {
    generate_message(b"PG;")
}

/// Generate stream reset command.
pub fn generate_stream_reset_command() -> Vec<u8> {
    generate_message(b"JS;")
}

/// Generate a Vevor USB protocol message with custom payload.
fn generate_message(payload: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(28 + payload.len());

    // Byte 0: ESC marker
    msg.push(0x1B);
    // Byte 1: Padding
    msg.push(0x00);

    // Bytes 2-5: Placeholder for CRC32 (will be calculated)
    msg.push(0x00);
    msg.push(0x00);
    msg.push(0x00);
    msg.push(0x00);

    // Bytes 6-23: Fixed constant bytes
    msg.extend_from_slice(&[
        0x09, 0x91, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x01, 0x03, 0x00, 0x18,
        0x00, 0x02, 0x03,
    ]);

    // Bytes 24-25: Payload length (little-endian u16)
    let len = payload.len() as u16;
    msg.extend_from_slice(&len.to_le_bytes());

    // Bytes 26-27: Padding
    msg.push(0x00);
    msg.push(0x00);

    // Calculate CRC32 from bytes 6 onwards (including header and payload)
    let crc_data_start = 6;
    let mut hasher = Hasher::new();
    hasher.update(&msg[crc_data_start..]);
    hasher.update(payload);
    let crc = hasher.finalize();

    // Insert CRC32 at bytes 2-5 (little-endian)
    msg[2..6].copy_from_slice(&crc.to_le_bytes());

    // Append payload
    msg.extend_from_slice(payload);

    msg
}

/// Parse a response from the Vevor device.
/// Extracts the payload from the binary protocol message and returns it as a UTF-8 string.
pub fn parse_device_response(data: &[u8]) -> Result<String, &'static str> {
    if data.len() < 28 {
        return Err("Response too short: insufficient header");
    }

    // Verify ESC marker
    if data[0] != 0x1B {
        return Err("Invalid response: missing ESC marker");
    }

    // Extract payload length from bytes 24-25 (little-endian)
    let payload_len = u16::from_le_bytes([data[24], data[25]]) as usize;

    // Verify we have enough data
    if data.len() < 28 + payload_len {
        return Err("Response truncated: insufficient payload data");
    }

    // Extract and decode payload
    let payload_start = 28;
    let payload = &data[payload_start..payload_start + payload_len];
    String::from_utf8(payload.to_vec()).map_err(|_| "Invalid UTF-8 in device response")
}

/// Extract event keywords from device response.
pub fn extract_event_keywords(response: &str) -> Vec<&str> {
    let keywords = [
        "LOADPRESSED",
        "LOADSUCCESS",
        "START",
        "BUSY",
        "IDLE",
        "CUTSUCCESS",
        "CUTOVER",
        "ERROR",
    ];

    keywords
        .iter()
        .filter(|&&kw| response.contains(kw))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hpgl_message_generation() {
        let hpgl = "IN;\r\nSP1;FS10;VS250;PU40,40;PD40,60;PU5875,1265;PD6117,1265;";
        let msg = generate_hpgl_message(hpgl);

        // Header should be 28 bytes
        assert!(msg.len() >= 28);

        // Byte 0 should be ESC
        assert_eq!(msg[0], 0x1B);

        // Bytes 6-23 should be the fixed constant
        let expected_fixed = &[
            0x09, 0x91, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x01, 0x03, 0x00, 0x18,
            0x00, 0x02, 0x03,
        ];
        assert_eq!(&msg[6..24], expected_fixed);

        // Bytes 24-25 should encode the HPGL payload length
        let encoded_len = u16::from_le_bytes([msg[24], msg[25]]) as usize;
        assert_eq!(encoded_len, hpgl.len());

        // The payload should be appended after the header
        let payload_start = 28;
        assert_eq!(&msg[payload_start..], hpgl.as_bytes());
    }
}
