//! Abstract device interface and the event types shared by all drivers.

/// Events emitted by a device worker and forwarded to the UI.
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    Connected,
    Disconnected,
    /// A raw response line from the machine.
    LineReceived(String),
    /// Machine position update (x_mm, y_mm).
    PositionUpdate(f64, f64),
    /// Job progress 0–100.
    JobProgress(u8),
    /// Job finished; `true` = success.
    JobFinished(bool),
    /// Status / error message.
    Message(String),
}

/// Commands sent from the UI to a device worker.
#[derive(Debug, Clone)]
pub enum DeviceCommand {
    Connect { port: String, baud_rate: u32 },
    Disconnect,
    SendJob(Vec<String>),
    FeedHold,
    CycleStart,
    SoftReset,
    Home,
    Jog { x: f64, y: f64, feed_mm_min: f64 },
    PollStatus,
}
