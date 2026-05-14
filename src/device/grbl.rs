//! GRBL laser/spindle driver.
//!
//! Runs a blocking I/O loop in a dedicated thread.  Commands arrive from the
//! application via a `tokio::sync::mpsc` channel; events are sent back the
//! same way.

use std::io::Write;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::device::base::{DeviceCommand, DeviceEvent};

/// Spawn a GRBL device worker.
///
/// Returns `(cmd_tx, event_rx)`.  Drop `cmd_tx` to stop the worker.
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
            let mut job_lines: Vec<String> = Vec::new();
            let mut job_idx = 0usize;
            let mut last_poll = Instant::now();

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    DeviceCommand::Connect { port: port_name, baud_rate } => {
                        match serialport::new(&port_name, baud_rate)
                            .timeout(Duration::from_secs(2))
                            .open()
                        {
                            Ok(mut p) => {
                                // Wait for GRBL init banner (~2 s)
                                std::thread::sleep(Duration::from_millis(2000));
                                let _ = p.write_all(b"\r\n");
                                port = Some(p);
                                let _ = event_tx.send(DeviceEvent::Connected).await;
                                let _ = event_tx
                                    .send(DeviceEvent::Message("GRBL connected".to_owned()))
                                    .await;
                            }
                            Err(e) => {
                                let _ = event_tx
                                    .send(DeviceEvent::Message(format!("Connection error: {e}")))
                                    .await;
                            }
                        }
                    }

                    DeviceCommand::Disconnect => {
                        port = None;
                        job_lines.clear();
                        job_idx = 0;
                        let _ = event_tx.send(DeviceEvent::Disconnected).await;
                    }

                    DeviceCommand::SendJob(lines) => {
                        job_lines = lines;
                        job_idx = 0;
                    }

                    DeviceCommand::FeedHold => {
                        if let Some(ref mut p) = port {
                            let _ = p.write_all(b"!");
                        }
                    }

                    DeviceCommand::CycleStart => {
                        if let Some(ref mut p) = port {
                            let _ = p.write_all(b"~");
                        }
                    }

                    DeviceCommand::SoftReset => {
                        if let Some(ref mut p) = port {
                            let _ = p.write_all(b"\x18");
                        }
                    }

                    DeviceCommand::Home => {
                        if let Some(ref mut p) = port {
                            let _ = writeln!(p, "$H");
                        }
                    }

                    DeviceCommand::Jog { x, y, feed_mm_min } => {
                        if let Some(ref mut p) = port {
                            let cmd = format!(
                                "$J=G91 G21 X{x:.3} Y{y:.3} F{feed_mm_min:.0}\n"
                            );
                            let _ = p.write_all(cmd.as_bytes());
                        }
                    }

                    DeviceCommand::PollStatus => {
                        if let Some(ref mut p) = port {
                            let _ = p.write_all(b"?");
                        }
                    }
                }

                // Drive the job and read responses
                if let Some(ref mut p) = port {
                    // Send next job line if pending
                    if job_idx < job_lines.len() {
                        let line = format!("{}\n", job_lines[job_idx]);
                        if p.write_all(line.as_bytes()).is_ok() {
                            job_idx += 1;
                            let pct = (job_idx * 100 / job_lines.len().max(1)) as u8;
                            let _ = event_tx.send(DeviceEvent::JobProgress(pct)).await;
                        }
                    } else if job_idx > 0 && job_idx == job_lines.len() {
                        let _ = event_tx.send(DeviceEvent::JobFinished(true)).await;
                        job_lines.clear();
                        job_idx = 0;
                    }

                    // Poll status
                    if last_poll.elapsed() >= Duration::from_secs(1) {
                        let _ = p.write_all(b"?");
                        last_poll = Instant::now();
                    }

                    // Non-blocking read
                    if let Ok(avail) = p.bytes_to_read() {
                        if avail > 0 {
                            let mut buf = vec![0u8; avail as usize];
                            if let Ok(n) = p.read(&mut buf) {
                                let text =
                                    String::from_utf8_lossy(&buf[..n]).to_string();
                                for line in text.lines() {
                                    let line = line.trim().to_owned();
                                    if !line.is_empty() {
                                        parse_grbl_response(
                                            &line,
                                            &event_tx,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    });

    (cmd_tx, event_rx)
}

async fn parse_grbl_response(line: &str, tx: &Sender<DeviceEvent>) {
    let _ = tx.send(DeviceEvent::LineReceived(line.to_owned())).await;

    // Parse real-time status: <Idle|MPos:0.000,0.000,0.000|...>
    if line.starts_with('<') {
        let inner = line.trim_matches(|c| c == '<' || c == '>');
        for part in inner.split('|') {
            if let Some(coords) = part.strip_prefix("MPos:") {
                let nums: Vec<&str> = coords.split(',').collect();
                if nums.len() >= 2 {
                    let x = nums[0].parse::<f64>().unwrap_or(0.0);
                    let y = nums[1].parse::<f64>().unwrap_or(0.0);
                    let _ = tx.send(DeviceEvent::PositionUpdate(x, y)).await;
                }
            }
        }
    } else if line.starts_with("error:") {
        let _ = tx
            .send(DeviceEvent::Message(format!("GRBL {line}")))
            .await;
    }
}
