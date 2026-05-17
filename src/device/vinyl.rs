//! Vinyl cutter driver – HPGL over serial (Roland, Graphtec, Silhouette, …).
//!
//! Plotter units: 1 unit = 1/40 mm (0.025 mm).
//!
//! Commands used:
//! * `IN`        – initialise / reset
//! * `SP<n>`     – select pen / blade
//! * `VS<n>`     – velocity / cutting speed (cm/s)
//! * `FS<n>`     – force in grams
//! * `PU<x>,<y>` – pen up + move
//! * `PD<x>,<y>` – pen down + cut

use std::io::Write;
use std::time::Duration;

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::device::base::{DeviceCommand, DeviceEvent};

/// Plotter units per mm.
const HPGL_PER_MM: f64 = 40.0;

/// Spawn a vinyl-cutter device worker.
///
/// Returns `(cmd_tx, event_rx)`.
pub fn spawn() -> (Sender<DeviceCommand>, Receiver<DeviceEvent>) {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<DeviceCommand>(64);
    let (event_tx, event_rx) = mpsc::channel::<DeviceEvent>(64);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async {
            let mut port: Option<Box<dyn serialport::SerialPort>> = None;

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    DeviceCommand::Connect { port: port_name, baud_rate } => {
                        match serialport::new(&port_name, baud_rate)
                            .timeout(Duration::from_secs(2))
                            // Many vinyl cutters use RTS/CTS flow control
                            .flow_control(serialport::FlowControl::Hardware)
                            .open()
                        {
                            Ok(mut p) => {
                                // Initialise cutter
                                let _ = p.write_all(b"IN;\r\n");
                                port = Some(p);
                                let _ = event_tx.send(DeviceEvent::Connected).await;
                                let _ = event_tx
                                    .send(DeviceEvent::Message(
                                        "Vinyl cutter connected".to_owned(),
                                    ))
                                    .await;
                            }
                            Err(e) => {
                                let _ = event_tx
                                    .send(DeviceEvent::Message(format!(
                                        "Connection error: {e}"
                                    )))
                                    .await;
                            }
                        }
                    }

                    DeviceCommand::Disconnect => {
                        port = None;
                        let _ = event_tx.send(DeviceEvent::Disconnected).await;
                    }

                    DeviceCommand::SendJob(lines) => {
                        if let Some(ref mut p) = port {
                            let total = lines.len();
                            for (i, line) in lines.iter().enumerate() {
                                let data =
                                    format!("{}\r\n", line.trim()).into_bytes();
                                if let Err(e) = p.write_all(&data) {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!(
                                            "Send error: {e}"
                                        )))
                                        .await;
                                    let _ =
                                        event_tx.send(DeviceEvent::JobFinished(false)).await;
                                    break;
                                }
                                // Small delay to avoid overrunning buffer
                                std::thread::sleep(Duration::from_millis(5));
                                let pct = ((i + 1) * 100 / total.max(1)) as u8;
                                let _ =
                                    event_tx.send(DeviceEvent::JobProgress(pct)).await;
                            }
                            let _ = event_tx.send(DeviceEvent::JobFinished(true)).await;
                        }
                    }

                    // Vinyl cutters have no feed-hold / cycle-start / home
                    DeviceCommand::FeedHold
                    | DeviceCommand::CycleStart
                    | DeviceCommand::SoftReset
                    | DeviceCommand::Home
                    | DeviceCommand::Jog { .. }
                    | DeviceCommand::SendRaw(_)
                    | DeviceCommand::PollStatus => {}
                }
            }
        });
    });

    (cmd_tx, event_rx)
}

// ---------------------------------------------------------------------------
// HPGL coordinate helpers (used by GCodeGenerator)
// ---------------------------------------------------------------------------

pub fn mm_to_hpgl(mm: f64) -> i32 {
    (mm * HPGL_PER_MM).round() as i32
}
