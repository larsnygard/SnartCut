//! Vevor Smart 1 vinyl cutter driver.
//!
//! The Vevor Smart 1 (and similar Vevor cutters) use standard HPGL but connect
//! via USB printer class rather than a serial port.
//!
//! # Connection
//!
//! * **Linux** – appears as `/dev/usb/lp0` (or `/dev/usb/lp1`, …).
//!   Write HPGL text directly to the file.
//! * **Windows** – typically shows as a USB printer; accessible via `LPT1:` or
//!   by printing to the raw printer queue.  A path like `\\.\USB001` or
//!   `\\.\LPT1` also works if exposed by the driver.
//! * **Serial fallback** – if the port path looks like a serial device
//!   (`/dev/ttyUSB*`, `COM*`) the driver falls back to the standard HPGL
//!   serial approach (no hardware flow control, 9600 baud default).
//!
//! # Protocol
//!
//! Standard HPGL (same subset as `vinyl.rs`):
//! * `IN;`             – initialise
//! * `SP1;`            – select blade/pen 1
//! * `VS<n>;`          – velocity (cm/s)
//! * `FS<n>;`          – blade force (grams)
//! * `PU<x>,<y>;`      – pen up + move
//! * `PD<x>,<y>;`      – pen down + cut
//!
//! Coordinate unit: 1 unit = 1/40 mm (0.025 mm) – identical to standard HPGL.
//!
//! # Work area (default preset)
//! Width: 304.8 mm (12 in), Height: 347.8 mm (~13.7 in)

use std::io::Write;
use std::time::Duration;

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::device::base::{DeviceCommand, DeviceEvent};

/// Default work area for the Vevor Smart 1.
pub const WORK_AREA_W_MM: f64 = 304.8;
pub const WORK_AREA_H_MM: f64 = 347.8;

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

enum Transport {
    /// File-based write (USB printer class: /dev/usb/lp*, LPT*, \\.\USB*)
    File(std::fs::File),
    /// Serial fallback
    Serial(Box<dyn serialport::SerialPort>),
}

impl Transport {
    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        match self {
            Transport::File(f) => f.write_all(data),
            Transport::Serial(p) => p.write_all(data),
        }
    }
}

fn is_usb_printer_path(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains("/usb/lp")
        || p.contains("lpt")
        || p.starts_with("\\\\.\\usb")
        || p.contains("usb001")
        || p.contains("usb002")
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

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

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    DeviceCommand::Connect { port: path, baud_rate } => {
                        let t = if is_usb_printer_path(&path) {
                            // USB printer class – open as a plain file
                            match std::fs::OpenOptions::new()
                                .write(true)
                                .open(&path)
                            {
                                Ok(f) => Some(Transport::File(f)),
                                Err(e) => {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!(
                                            "Cannot open {path}: {e}"
                                        )))
                                        .await;
                                    None
                                }
                            }
                        } else {
                            // Serial fallback (no hardware flow control for Vevor)
                            match serialport::new(&path, baud_rate)
                                .timeout(Duration::from_secs(2))
                                .flow_control(serialport::FlowControl::None)
                                .open()
                            {
                                Ok(p) => Some(Transport::Serial(p)),
                                Err(e) => {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!(
                                            "Connection error: {e}"
                                        )))
                                        .await;
                                    None
                                }
                            }
                        };

                        if let Some(t) = t {
                            // Initialise the cutter
                            let mut t = t;
                            let _ = t.write_all(b"IN;\r\n");
                            transport = Some(t);
                            let _ = event_tx.send(DeviceEvent::Connected).await;
                            let _ = event_tx
                                .send(DeviceEvent::Message(
                                    "Vevor Smart 1 connected".to_owned(),
                                ))
                                .await;
                        }
                    }

                    DeviceCommand::Disconnect => {
                        transport = None;
                        let _ = event_tx.send(DeviceEvent::Disconnected).await;
                    }

                    DeviceCommand::SendJob(lines) => {
                        if let Some(ref mut t) = transport {
                            let total = lines.len();
                            for (i, line) in lines.iter().enumerate() {
                                let data = format!("{}\r\n", line.trim()).into_bytes();
                                if let Err(e) = t.write_all(&data) {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!("Send error: {e}")))
                                        .await;
                                    let _ = event_tx.send(DeviceEvent::JobFinished(false)).await;
                                    break;
                                }
                                // Small inter-command delay to avoid overrunning the USB buffer
                                std::thread::sleep(Duration::from_millis(5));
                                let pct = ((i + 1) * 100 / total.max(1)) as u8;
                                let _ = event_tx.send(DeviceEvent::JobProgress(pct)).await;
                            }
                            let _ = event_tx.send(DeviceEvent::JobFinished(true)).await;
                        }
                    }

                    // Vevor has no feed-hold / cycle-start / homing
                    DeviceCommand::FeedHold
                    | DeviceCommand::CycleStart
                    | DeviceCommand::SoftReset
                    | DeviceCommand::Home
                    | DeviceCommand::Jog { .. }
                    | DeviceCommand::PollStatus => {}
                }
            }
        });
    });

    (cmd_tx, event_rx)
}
