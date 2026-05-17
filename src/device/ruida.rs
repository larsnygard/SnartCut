//! Ruida laser controller driver.
//!
//! Ruida controllers (RDC6442G, RDC6445, RDC6332, …) are found in most
//! Chinese CO₂ and diode laser cutters (Thunder Laser, Boss Laser, OMTech, …).
//!
//! # Communication
//!
//! Ruida supports two transports:
//! * **UDP** – controller listens on `<machine_ip>:50200`, host uses
//!   `<machine_ip>:50300` as the source/dest pair. This is how LightBurn and
//!   RDWorks communicate.
//! * **USB-CDC serial** – appears as a standard serial port.
//!
//! The port string in the device profile selects the transport:
//! * An IP address (e.g. `192.168.1.100`) → UDP on port 50200.
//! * A path (e.g. `/dev/ttyUSB0`, `COM3`) → serial at the configured baud.
//!
//! # Protocol
//!
//! Ruida uses a scrambled binary protocol.  Every byte is XOR-scrambled with
//! key 0x88 before transmission, and the machine scrambles its responses the
//! same way.
//!
//! Packet structure (simplified):
//! ```text
//! [cmd_byte] [data…]
//! ```
//! Packets are framed with a simple checksum.  For job transfer the host
//! sends `.rd` file data wrapped in `0xD4` / `0xD5` transfer commands.
//!
//! This driver implements:
//! * Ping / heartbeat (`0xDA 0x00`)
//! * Get position (`0xD8 0x00`)
//! * Job transfer using `ruida_file` bytes wrapped in Begin/End framing
//! * Jog (`0xD9 xx yy`)
//! * Feed hold / resume / reset (same bytes as the panel keys)

use std::io::{Read, Write};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::device::base::{DeviceCommand, DeviceEvent};

// ---------------------------------------------------------------------------
// Protocol constants
// ---------------------------------------------------------------------------

const XOR_KEY: u8 = 0x88;

// Command bytes
const CMD_PING:        u8 = 0xDA;
const CMD_GET_POS:     u8 = 0xD8;
const CMD_JOG:         u8 = 0xD9;
const CMD_FEED_HOLD:   u8 = 0x24;
const CMD_CYCLE_START: u8 = 0x25;
const CMD_SOFT_RESET:  u8 = 0x26;
const CMD_HOME:        u8 = 0xD8; // 0xD8 0x2C
const CMD_FILE_BEGIN:  u8 = 0xD4;
const CMD_FILE_END:    u8 = 0xD5;
const CMD_FILE_DATA:   u8 = 0xD3;

const UDP_MACHINE_PORT: u16 = 50200;
const UDP_LOCAL_PORT:   u16 = 50300;

// ---------------------------------------------------------------------------
// Scrambling helpers
// ---------------------------------------------------------------------------

/// XOR-scramble a buffer in-place.
fn scramble(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        *b ^= XOR_KEY;
    }
}

/// Build a scrambled packet with a simple trailing checksum.
/// Ruida checksum = lower byte of sum of all data bytes before scrambling.
fn make_packet(cmd: u8, data: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(1 + data.len() + 1);
    raw.push(cmd);
    raw.extend_from_slice(data);
    let checksum: u8 = raw.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    raw.push(checksum);
    scramble(&mut raw);
    raw
}

/// Encode a coordinate in Ruida's native unit (1 unit = 1 µm = 0.001 mm).
/// Returns 4 big-endian bytes.
fn encode_coord(mm: f64) -> [u8; 4] {
    let units = (mm * 1000.0).round() as u32;
    units.to_be_bytes()
}

// ---------------------------------------------------------------------------
// Transport abstraction
// ---------------------------------------------------------------------------

enum Transport {
    Udp {
        socket: UdpSocket,
        remote: SocketAddr,
    },
    Serial(Box<dyn serialport::SerialPort>),
}

impl Transport {
    fn send(&mut self, pkt: &[u8]) -> std::io::Result<()> {
        match self {
            Transport::Udp { socket, remote } => {
                socket.send_to(pkt, *remote)?;
            }
            Transport::Serial(p) => {
                p.write_all(pkt)?;
            }
        }
        Ok(())
    }

    /// Try a non-blocking read; returns bytes read (0 if nothing available).
    fn try_recv(&mut self, buf: &mut [u8]) -> usize {
        match self {
            Transport::Udp { socket, .. } => {
                socket.set_nonblocking(true).ok();
                match socket.recv(buf) {
                    Ok(n) => n,
                    Err(_) => 0,
                }
            }
            Transport::Serial(p) => {
                match p.bytes_to_read() {
                    Ok(avail) if avail > 0 => {
                        let to_read = (avail as usize).min(buf.len());
                        p.read(&mut buf[..to_read]).unwrap_or(0)
                    }
                    _ => 0,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

/// Spawn a Ruida device worker.
///
/// The `port` string in `DeviceCommand::Connect` may be:
/// * An IPv4 address (`192.168.1.100`) → UDP transport
/// * A serial device path (`/dev/ttyUSB0`, `COM3`) → serial transport
pub fn spawn() -> (Sender<DeviceCommand>, Receiver<DeviceEvent>) {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<DeviceCommand>(64);
    let (event_tx, event_rx) = mpsc::channel::<DeviceEvent>(64);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async {
            let mut transport: Option<Transport> = None;
            let mut last_ping = Instant::now();

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    DeviceCommand::Connect { port: addr, baud_rate } => {
                        let t = if looks_like_ip(&addr) {
                            // UDP transport
                            let local: SocketAddr =
                                format!("0.0.0.0:{UDP_LOCAL_PORT}").parse().unwrap();
                            match UdpSocket::bind(local) {
                                Ok(sock) => {
                                    let _ = sock.set_read_timeout(
                                        Some(Duration::from_millis(200))
                                    );
                                    let remote: SocketAddr =
                                        format!("{addr}:{UDP_MACHINE_PORT}").parse().unwrap();
                                    Some(Transport::Udp { socket: sock, remote })
                                }
                                Err(e) => {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!(
                                            "UDP bind error: {e}"
                                        )))
                                        .await;
                                    None
                                }
                            }
                        } else {
                            // Serial transport
                            match serialport::new(&addr, baud_rate)
                                .timeout(Duration::from_millis(500))
                                .open()
                            {
                                Ok(p) => Some(Transport::Serial(p)),
                                Err(e) => {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!(
                                            "Serial error: {e}"
                                        )))
                                        .await;
                                    None
                                }
                            }
                        };

                        if let Some(t) = t {
                            transport = Some(t);
                            let _ = event_tx.send(DeviceEvent::Connected).await;
                            let _ = event_tx
                                .send(DeviceEvent::Message("Ruida connected".to_owned()))
                                .await;
                        }
                    }

                    DeviceCommand::Disconnect => {
                        transport = None;
                        let _ = event_tx.send(DeviceEvent::Disconnected).await;
                    }

                    DeviceCommand::SendJob(lines) => {
                        if let Some(ref mut t) = transport {
                            // `lines` here contains the `.rd` binary data encoded as
                            // hex lines (one byte per line) or raw byte strings.
                            // For Ruida we expect the generator to produce hex-encoded bytes.
                            let bytes = decode_rd_lines(&lines);
                            send_job(t, &bytes, &event_tx).await;
                        }
                    }

                    DeviceCommand::FeedHold => {
                        if let Some(ref mut t) = transport {
                            let pkt = make_packet(CMD_FEED_HOLD, &[]);
                            let _ = t.send(&pkt);
                        }
                    }

                    DeviceCommand::CycleStart => {
                        if let Some(ref mut t) = transport {
                            let pkt = make_packet(CMD_CYCLE_START, &[]);
                            let _ = t.send(&pkt);
                        }
                    }

                    DeviceCommand::SoftReset => {
                        if let Some(ref mut t) = transport {
                            let pkt = make_packet(CMD_SOFT_RESET, &[]);
                            let _ = t.send(&pkt);
                        }
                    }

                    DeviceCommand::Home => {
                        if let Some(ref mut t) = transport {
                            // 0xD8 0x2C = return to origin
                            let pkt = make_packet(CMD_HOME, &[0x2C]);
                            let _ = t.send(&pkt);
                        }
                    }

                    DeviceCommand::Jog { x, y, feed_mm_min: _ } => {
                        if let Some(ref mut t) = transport {
                            let mut data = Vec::with_capacity(9);
                            data.push(0x01); // relative move
                            data.extend_from_slice(&encode_coord(x));
                            data.extend_from_slice(&encode_coord(y));
                            let pkt = make_packet(CMD_JOG, &data);
                            let _ = t.send(&pkt);
                        }
                    }

                    DeviceCommand::PollStatus => {
                        if let Some(ref mut t) = transport {
                            let pkt = make_packet(CMD_GET_POS, &[]);
                            let _ = t.send(&pkt);
                        }
                    }

                    DeviceCommand::SendRaw(_) => {}
                }

                // Periodic heartbeat & response processing
                if let Some(ref mut t) = transport {
                    if last_ping.elapsed() >= Duration::from_secs(2) {
                        let pkt = make_packet(CMD_PING, &[0x00]);
                        let _ = t.send(&pkt);
                        last_ping = Instant::now();
                    }

                    let mut buf = [0u8; 64];
                    let n = t.try_recv(&mut buf);
                    if n > 0 {
                        let mut raw = buf[..n].to_vec();
                        scramble(&mut raw); // unscramble
                        parse_ruida_response(&raw, &event_tx).await;
                    }
                }
            }
        });
    });

    (cmd_tx, event_rx)
}

// ---------------------------------------------------------------------------
// Job transfer
// ---------------------------------------------------------------------------

async fn send_job(t: &mut Transport, data: &[u8], event_tx: &Sender<DeviceEvent>) {
    // Begin transfer
    let begin = make_packet(CMD_FILE_BEGIN, &[]);
    if t.send(&begin).is_err() {
        let _ = event_tx.send(DeviceEvent::Message("Transfer error".to_owned())).await;
        return;
    }

    // Send data in 500-byte chunks
    const CHUNK: usize = 500;
    let total = data.len();
    let mut sent = 0;

    for chunk in data.chunks(CHUNK) {
        let mut payload = Vec::with_capacity(chunk.len());
        payload.extend_from_slice(chunk);
        let pkt = make_packet(CMD_FILE_DATA, &payload);
        if t.send(&pkt).is_err() {
            let _ = event_tx.send(DeviceEvent::Message("Transfer error".to_owned())).await;
            return;
        }
        sent += chunk.len();
        let pct = (sent * 100 / total.max(1)) as u8;
        let _ = event_tx.send(DeviceEvent::JobProgress(pct)).await;
    }

    // End transfer
    let end = make_packet(CMD_FILE_END, &[]);
    let _ = t.send(&end);
    let _ = event_tx.send(DeviceEvent::JobFinished(true)).await;
}

// ---------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------

async fn parse_ruida_response(data: &[u8], tx: &Sender<DeviceEvent>) {
    if data.is_empty() {
        return;
    }

    let summary = data
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    let _ = tx.send(DeviceEvent::LineReceived(summary)).await;

    // Position response: [D8 00] x[4] y[4]
    if data.len() >= 9 && data[0] == CMD_GET_POS {
        let x = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as f64 / 1000.0;
        let y = u32::from_be_bytes([data[5], data[6], data[7], data[8]]) as f64 / 1000.0;
        let _ = tx.send(DeviceEvent::PositionUpdate(x, y)).await;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn looks_like_ip(s: &str) -> bool {
    s.parse::<std::net::IpAddr>().is_ok()
}

/// Decode `.rd` data from hex-per-line strings produced by the generator.
fn decode_rd_lines(lines: &[String]) -> Vec<u8> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let trimmed = line.trim();
        if let Ok(b) = u8::from_str_radix(trimmed, 16) {
            out.push(b);
        }
    }
    out
}
