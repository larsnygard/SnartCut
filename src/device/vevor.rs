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

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Color, Element, Length};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::app::Message;
use crate::device::base::{DeviceCommand, DeviceEvent};
use crate::device::vevor_usb_protocol;

/// Default work area for the Vevor Smart 1.
pub const WORK_AREA_W_MM: f64 = 304.8;
pub const WORK_AREA_H_MM: f64 = 347.8;

// ---------------------------------------------------------------------------
// Modular device control panel
// ---------------------------------------------------------------------------

/// Returns the Vevor-specific hardware control panel for embedding in the device panel.
pub fn controls_view<'a>(connected: bool, last_status: &'a str) -> Element<'a, Message> {
    let btn_style = |bg: Color| {
        move |_t: &iced::Theme, _s: button::Status| button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::WHITE,
            border: iced::Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        }
    };

    let dim = Color::from_rgb(0.25, 0.25, 0.25);
    let blue = Color::from_rgb(0.1, 0.35, 0.65);
    let teal = Color::from_rgb(0.1, 0.5, 0.5);
    let orange = Color::from_rgb(0.6, 0.35, 0.0);

    let setmat_btn = button(text("setmat").size(11))
        .on_press_maybe(if connected { Some(Message::VevorSendSetmat) } else { None })
        .style(btn_style(if connected { blue } else { dim }));

    let tb42_btn = button(text("TB42 Status").size(11))
        .on_press_maybe(if connected { Some(Message::VevorPollStatus) } else { None })
        .style(btn_style(if connected { teal } else { dim }));

    let send_job_btn = button(text("▶ Send Job").size(11))
        .on_press_maybe(if connected { Some(Message::SendJob) } else { None })
        .style(btn_style(if connected { Color::from_rgb(0.1, 0.55, 0.1) } else { dim }));

    let sync_reset_btn = button(text("Sync Reset").size(11))
        .on_press_maybe(if connected { Some(Message::VevorSyncReset) } else { None })
        .style(btn_style(if connected { orange } else { dim }));

    let status_display = row![
        text("Status:").size(11).style(|_: &iced::Theme| text::Style {
            color: Some(Color::from_rgb(0.6, 0.6, 0.6)),
        }),
        text(if last_status.is_empty() { "—" } else { last_status })
            .size(11)
            .style(|_: &iced::Theme| text::Style {
                color: Some(Color::from_rgb(0.9, 0.85, 0.5)),
            }),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let controls_row = row![setmat_btn, tb42_btn, send_job_btn, sync_reset_btn]
        .spacing(4)
        .align_y(Alignment::Center);

    let label = text("Vevor Controls").size(10).style(|_: &iced::Theme| text::Style {
        color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
    });

    let col = column![label, controls_row, status_display].spacing(3);

    container(col)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.14))),
            border: iced::Border {
                color: Color::from_rgb(0.25, 0.25, 0.3),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .padding(6)
        .width(Length::Fill)
        .into()
}

#[cfg(target_os = "windows")]
pub fn auto_detect_windows_target() -> Option<String> {
    use std::process::Command;

    // Most reliable path for this device family: match printer by PNP VID/PID.
    // smart-pcap.json confirms Vevor Smart cutter as VID_045B & PID_5310.
    let vid_pid_queue_cmd = "Get-CimInstance Win32_Printer | Where-Object { $_.PNPDeviceID -match 'VID_045B&PID_5310' -or $_.PNPDeviceID -match 'VID_045B.*PID_5310' } | Select-Object -First 1 | ForEach-Object { \"Printer Queue: $($_.Name) [$($_.PortName)]\" }";
    let vid_pid_queue_out = Command::new("powershell")
        .args(["-NoProfile", "-Command", vid_pid_queue_cmd])
        .output()
        .ok();

    if let Some(out) = vid_pid_queue_out {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::trim)
                .find(|s| !s.is_empty())
            {
                return Some(line.to_owned());
            }
        }
    }

    // Prefer explicit printer queues first.
    let queue_cmd = "Get-Printer | Where-Object { $_.Name -match 'PosteK|POSTEK|Q8/200|Vevor' -or $_.DriverName -match 'PosteK|POSTEK|Q8/200|Vevor' } | Select-Object -First 1 | ForEach-Object { \"Printer Queue: $($_.Name) [$($_.PortName)]\" }";
    let queue_out = Command::new("powershell")
        .args(["-NoProfile", "-Command", queue_cmd])
        .output()
        .ok();

    if let Some(out) = queue_out {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::trim)
                .find(|s| !s.is_empty())
            {
                return Some(line.to_owned());
            }
        }
    }

    // Fallback to any printer on USBnnn port.
    let usb_queue_cmd = "Get-Printer | Where-Object { $_.PortName -match 'USB\\d{3}' } | Select-Object -First 1 | ForEach-Object { \"Printer Queue: $($_.Name) [$($_.PortName)]\" }";
    let usb_queue_out = Command::new("powershell")
        .args(["-NoProfile", "-Command", usb_queue_cmd])
        .output()
        .ok();

    if let Some(out) = usb_queue_out {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::trim)
                .find(|s| !s.is_empty())
            {
                return Some(line.to_owned());
            }
        }
    }

    // Generic USB printer fallback: any local USB printer-class queue.
    let class_queue_cmd = "Get-CimInstance Win32_Printer | Where-Object { $_.PortName -match '^USB\\d{3}$' -or $_.PNPDeviceID -like 'USBPRINT*' } | Select-Object -First 1 | ForEach-Object { \"Printer Queue: $($_.Name) [$($_.PortName)]\" }";
    let class_queue_out = Command::new("powershell")
        .args(["-NoProfile", "-Command", class_queue_cmd])
        .output()
        .ok();

    if let Some(out) = class_queue_out {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(str::trim)
                .find(|s| !s.is_empty())
            {
                return Some(line.to_owned());
            }
        }
    }

    // Last resort: scan PnP software device and emit direct USB candidate.
    let pnp_cmd = "Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -like 'USBPRINT*' -or $_.InstanceId -match 'VID_045B&PID_5310' -or $_.FriendlyName -match 'PosteK|POSTEK|Q8/200|Vevor' } | Select-Object -First 1 | ForEach-Object { \"$($_.InstanceId)||$($_.FriendlyName)\" }";
    let pnp_out = Command::new("powershell")
        .args(["-NoProfile", "-Command", pnp_cmd])
        .output()
        .ok()?;
    if !pnp_out.status.success() {
        return None;
    }
    let joined = String::from_utf8_lossy(&pnp_out.stdout)
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty())?
        .to_owned();

    let mut parts = joined.splitn(2, "||");
    let instance = parts.next().unwrap_or_default().trim().to_owned();
    let friendly = parts.next().unwrap_or_default().trim().to_owned();

    if let Some(token) = extract_usb_port_token(&instance) {
        return Some(format!("USB Path: \\\\.\\{token}"));
    }

    if !instance.is_empty() {
        if friendly.is_empty() {
            return Some(format!("USB PnP: {instance}"));
        }
        return Some(format!("USB PnP: {instance} | {friendly}"));
    }

    None
}

#[cfg(not(target_os = "windows"))]
pub fn auto_detect_windows_target() -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

enum Transport {
    /// File-based write (USB printer class: /dev/usb/lp*, LPT*, \\.\USB*)
    File(std::fs::File),
    /// Serial fallback
    Serial(Box<dyn serialport::SerialPort>),
    #[cfg(target_os = "windows")]
    /// Raw printer queue by queue name.
    PrinterQueue(String),
    #[cfg(target_os = "windows")]
    /// Direct USBPRINT interface path (SetupAPI-resolved)
    UsbPrintInterface(String),
}

impl Transport {
    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        match self {
            Transport::File(f) => f.write_all(data),
            Transport::Serial(p) => p.write_all(data),
            #[cfg(target_os = "windows")]
            Transport::PrinterQueue(name) => write_raw_to_printer_queue(name, data),
            #[cfg(target_os = "windows")]
            Transport::UsbPrintInterface(path) => write_raw_to_usbprint_interface(path, data),
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

#[cfg(target_os = "windows")]
fn extract_usb_port_token(path: &str) -> Option<String> {
    let up = path.to_uppercase();
    let bytes = up.as_bytes();
    for i in 0..bytes.len().saturating_sub(6) {
        if &bytes[i..i + 3] == b"USB" {
            let d0 = bytes.get(i + 3).copied();
            let d1 = bytes.get(i + 4).copied();
            let d2 = bytes.get(i + 5).copied();
            if matches!(d0, Some(b'0'..=b'9'))
                && matches!(d1, Some(b'0'..=b'9'))
                && matches!(d2, Some(b'0'..=b'9'))
            {
                return Some(up[i..i + 6].to_owned());
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn resolve_printer_queue_for_port(port: &str) -> Option<String> {
    use std::process::Command;

    let cmd = format!(
        "Get-Printer | Where-Object {{ $_.PortName -eq '{}' }} | Select-Object -First 1 -ExpandProperty Name",
        port
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", cmd.as_str()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let name = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty())?
        .to_owned();
    Some(name)
}

#[cfg(target_os = "windows")]
fn resolve_printer_queue_for_vid_pid() -> Option<String> {
    use std::process::Command;

    let cmd = "Get-CimInstance Win32_Printer | Where-Object { $_.PNPDeviceID -match 'VID_045B&PID_5310' -or $_.PNPDeviceID -match 'VID_045B.*PID_5310' } | Select-Object -First 1 -ExpandProperty Name";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", cmd])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(target_os = "windows")]
fn parse_printer_queue_name(path: &str) -> Option<String> {
    let p = path.trim();
    let pref = "Printer Queue: ";
    if !p.starts_with(pref) {
        return None;
    }
    let rest = &p[pref.len()..];
    let name = rest
        .split("[")
        .next()
        .unwrap_or(rest)
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

#[cfg(target_os = "windows")]
fn parse_usb_pnp_instance(path: &str) -> Option<String> {
    let p = path.trim();
    let pref = "USB PnP: ";
    if !p.starts_with(pref) {
        return None;
    }
    let rest = &p[pref.len()..];
    let instance = rest.split(" | ").next().unwrap_or(rest).trim();
    if instance.is_empty() {
        None
    } else {
        Some(instance.to_owned())
    }
}

#[cfg(target_os = "windows")]
fn resolve_usbprint_interface_path_for_vid_pid() -> Option<String> {
    use std::ptr;
    use windows_sys::core::GUID;
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
        SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
        DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
        SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
    };
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;

    // GUID_DEVINTERFACE_USBPRINT
    let usbprint_guid = GUID {
        data1: 0x28d78fad,
        data2: 0x5a12,
        data3: 0x11d1,
        data4: [0xae, 0x5b, 0x00, 0x00, 0xf8, 0x03, 0xa8, 0xc2],
    };

    let dev_info: HDEVINFO = unsafe {
        SetupDiGetClassDevsW(
            &usbprint_guid,
            ptr::null(),
            ptr::null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };

    if dev_info == INVALID_HANDLE_VALUE as HDEVINFO {
        return None;
    }

    let mut idx = 0;
    let mut found: Option<String> = None;
    loop {
        let mut if_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            InterfaceClassGuid: usbprint_guid,
            Flags: 0,
            Reserved: 0,
        };

        let ok = unsafe {
            SetupDiEnumDeviceInterfaces(
                dev_info,
                ptr::null_mut(),
                &usbprint_guid,
                idx,
                &mut if_data,
            )
        };
        if ok == 0 {
            break;
        }

        let mut required_size = 0u32;
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &if_data,
                ptr::null_mut(),
                0,
                &mut required_size,
                ptr::null_mut(),
            );
        }

        if required_size > 0 {
            let mut buf = vec![0u8; required_size as usize];
            let detail_ptr = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;

            unsafe {
                (*detail_ptr).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            }

            let ok_detail = unsafe {
                SetupDiGetDeviceInterfaceDetailW(
                    dev_info,
                    &if_data,
                    detail_ptr,
                    required_size,
                    &mut required_size,
                    ptr::null_mut(),
                )
            };

            if ok_detail != 0 {
                let path = unsafe {
                    let path_ptr = (*detail_ptr).DevicePath.as_ptr();
                    let mut len = 0usize;
                    while *path_ptr.add(len) != 0 {
                        len += 1;
                    }
                    String::from_utf16_lossy(std::slice::from_raw_parts(path_ptr, len))
                };

                let lower = path.to_lowercase();
                if lower.contains("vid_045b") && lower.contains("pid_5310") {
                    found = Some(path);
                    break;
                }
                if found.is_none() {
                    found = Some(path);
                }
            }
        }

        idx += 1;
    }

    unsafe {
        SetupDiDestroyDeviceInfoList(dev_info);
    }
    found
}

#[cfg(target_os = "windows")]
fn write_raw_to_usbprint_interface(device_path: &str, data: &[u8]) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(device_path)
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Failed to open USBPRINT interface '{device_path}': {e}"),
            )
        })?;

    file.write_all(data).map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("Failed to write data to USBPRINT interface '{device_path}': {e}"),
        )
    })
}

#[cfg(target_os = "windows")]
fn write_raw_to_printer_queue(queue_name: &str, data: &[u8]) -> std::io::Result<()> {
    use std::ffi::c_void;
    use std::io::{Error, ErrorKind};
    use windows_sys::Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW,
        StartPagePrinter, WritePrinter, DOC_INFO_1W,
    };

    let mut wide: Vec<u16> = queue_name.encode_utf16().collect();
    wide.push(0);

    let mut doc_name: Vec<u16> = "SnartCut Job".encode_utf16().collect();
    doc_name.push(0);
    let mut data_type: Vec<u16> = "RAW".encode_utf16().collect();
    data_type.push(0);

    let mut h_printer = std::ptr::null_mut();
    unsafe {
        if OpenPrinterW(wide.as_mut_ptr(), &mut h_printer, std::ptr::null_mut()) == 0
            || h_printer.is_null()
        {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to open printer queue '{queue_name}'"),
            ));
        }

        let doc = DOC_INFO_1W {
            pDocName: doc_name.as_mut_ptr(),
            pOutputFile: std::ptr::null_mut(),
            pDatatype: data_type.as_mut_ptr(),
        };

        if StartDocPrinterW(h_printer, 1, &doc) == 0 {
            ClosePrinter(h_printer);
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to start RAW print document on '{queue_name}'"),
            ));
        }

        if StartPagePrinter(h_printer) == 0 {
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to start print page on '{queue_name}'"),
            ));
        }

        let mut written: u32 = 0;
        if WritePrinter(
            h_printer,
            data.as_ptr() as *const c_void,
            data.len() as u32,
            &mut written,
        ) == 0
            || written == 0
        {
            EndPagePrinter(h_printer);
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to write RAW data to '{queue_name}'"),
            ));
        }

        EndPagePrinter(h_printer);
        EndDocPrinter(h_printer);
        ClosePrinter(h_printer);
    }

    Ok(())
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
                        #[cfg(target_os = "windows")]
                        let maybe_queue = parse_printer_queue_name(&path);

                        #[cfg(target_os = "windows")]
                        let usb_port = extract_usb_port_token(&path);

                        #[cfg(target_os = "windows")]
                        let maybe_pnp = parse_usb_pnp_instance(&path);

                        let t = if let Some(queue) = {
                            #[cfg(target_os = "windows")]
                            {
                                maybe_queue
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                None
                            }
                        } {
                            Some(Transport::PrinterQueue(queue))
                        } else if let Some(queue) = {
                            #[cfg(target_os = "windows")]
                            {
                                maybe_pnp
                                    .as_deref()
                                    .and_then(|_| resolve_printer_queue_for_vid_pid())
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                None
                            }
                        } {
                            Some(Transport::PrinterQueue(queue))
                        } else if let Some(interface_path) = {
                            #[cfg(target_os = "windows")]
                            {
                                maybe_pnp
                                    .as_deref()
                                    .and_then(|_| resolve_usbprint_interface_path_for_vid_pid())
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                None
                            }
                        } {
                            Some(Transport::UsbPrintInterface(interface_path))
                        } else if let Some(queue) = {
                            #[cfg(target_os = "windows")]
                            {
                                usb_port
                                    .as_deref()
                                    .and_then(resolve_printer_queue_for_port)
                            }
                            #[cfg(not(target_os = "windows"))]
                            {
                                None
                            }
                        } {
                            Some(Transport::PrinterQueue(queue))
                        } else if is_usb_printer_path(&path) {
                            // USB printer class – open as a plain file
                            let mut opened = std::fs::OpenOptions::new().write(true).open(&path);

                            #[cfg(target_os = "windows")]
                            if opened.is_err() {
                                if let Some(port) = usb_port.as_deref() {
                                    let alt = format!("\\\\.\\{port}");
                                    opened = std::fs::OpenOptions::new().write(true).open(&alt);
                                }
                            }

                            match opened {
                                Ok(f) => Some(Transport::File(f)),
                                Err(e) => {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!(
                                            "Vevor connect failed for '{path}': {e}. No usable printer queue found. Install/enable a printer queue for VID_045B&PID_5310, then select 'Printer Queue: ...'."
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
                                            "Connection error: {e}. If this is Vevor USB, select a 'Printer Queue: ...' target instead of serial."
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
                            #[cfg(target_os = "windows")]
                            if matches!(t, Transport::PrinterQueue(_)) {
                                // Vevor Smart1 USB stateful workflow:
                                // 1. Send "prepare for loading" command
                                // 2. Poll for LOADPRESSED
                                // 3. Wait for LOADSUCCESS
                                // 4. Wait for START
                                // 5. Send HPGL data in chunks
                                // 6. Poll for BUSY to clear
                                // 7. Wait for CUTSUCCESS
                                // 8. Wait for CUTOVER
                                // 9. Send stream reset

                                let _ = event_tx
                                    .send(DeviceEvent::Message(
                                        "Vevor: sending prepare-for-loading command...".to_owned(),
                                    ))
                                    .await;

                                // Step 1: Send prepare command
                                let prep_w = (WORK_AREA_W_MM * 40.0).round().max(1.0) as u32;
                                let prep_h = (WORK_AREA_H_MM * 40.0).round().max(1.0) as u32;
                                let prep_msg = vevor_usb_protocol::generate_prepare_job_with_size(1, prep_w, prep_h);
                                if let Err(e) = t.write_all(&prep_msg) {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!("Prepare error: {e}")))
                                        .await;
                                    let _ = event_tx.send(DeviceEvent::JobFinished(false)).await;
                                    continue;
                                }

                                // Step 2: Poll for LOADPRESSED (with timeout)
                                let _ = event_tx
                                    .send(DeviceEvent::Message(
                                        "Vevor: waiting for media load...".to_owned(),
                                    ))
                                    .await;
                                let mut load_ok = false;
                                for attempt in 0..60 {
                                    std::thread::sleep(Duration::from_millis(500));
                                    let poll_msg = vevor_usb_protocol::generate_status_poll_command();
                                    if let Err(e) = t.write_all(&poll_msg) {
                                        let _ = event_tx
                                            .send(DeviceEvent::Message(format!("Poll error: {e}")))
                                            .await;
                                        break;
                                    }
                                    std::thread::sleep(Duration::from_millis(100));
                                    // (Response handling would require reading from device; 
                                    // for now we use timeouts)
                                    if attempt % 10 == 0 {
                                        let _ = event_tx
                                            .send(DeviceEvent::Message(format!(
                                                "Vevor: waiting for media load ({} s)...",
                                                attempt / 2
                                            )))
                                            .await;
                                    }
                                    if attempt >= 6 {
                                        // Assume media is loaded after 3 seconds
                                        load_ok = true;
                                        break;
                                    }
                                }

                                if !load_ok {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(
                                            "Vevor: media load timeout".to_owned(),
                                        ))
                                        .await;
                                    let _ = event_tx.send(DeviceEvent::JobFinished(false)).await;
                                    continue;
                                }

                                // Step 5: Send HPGL data in chunks
                                let _ = event_tx
                                    .send(DeviceEvent::Message(
                                        "Vevor: sending HPGL data...".to_owned(),
                                    ))
                                    .await;

                                let mut hpgl = String::new();
                                for line in &lines {
                                    hpgl.push_str(line.trim());
                                    hpgl.push_str("\r\n");
                                }

                                // Send HPGL in chunks to avoid overwhelming the device
                                const CHUNK_SIZE: usize = 256;
                                let hpgl_bytes = hpgl.as_bytes();
                                let total_chunks = (hpgl_bytes.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;

                                for (chunk_idx, chunk) in hpgl_bytes.chunks(CHUNK_SIZE).enumerate() {
                                    let chunk_msg = vevor_usb_protocol::generate_hpgl_message(
                                        &String::from_utf8_lossy(chunk),
                                    );
                                    if let Err(e) = t.write_all(&chunk_msg) {
                                        let _ = event_tx
                                            .send(DeviceEvent::Message(format!(
                                                "HPGL send error: {e}"
                                            )))
                                            .await;
                                        let _ = event_tx.send(DeviceEvent::JobFinished(false)).await;
                                        break;
                                    }
                                    let pct = ((chunk_idx + 1) * 100 / total_chunks.max(1)) as u8;
                                    let _ = event_tx.send(DeviceEvent::JobProgress(pct)).await;
                                    std::thread::sleep(Duration::from_millis(50));
                                }

                                // Step 7: Wait for CUTSUCCESS
                                let _ = event_tx
                                    .send(DeviceEvent::Message(
                                        "Vevor: waiting for cut to complete...".to_owned(),
                                    ))
                                    .await;
                                std::thread::sleep(Duration::from_secs(2));

                                // Step 8: Wait for CUTOVER after eject
                                let _ = event_tx
                                    .send(DeviceEvent::Message(
                                        "Vevor: ejecting media...".to_owned(),
                                    ))
                                    .await;
                                let eject_msg = vevor_usb_protocol::generate_eject_command();
                                let _ = t.write_all(&eject_msg);
                                std::thread::sleep(Duration::from_secs(1));

                                // Step 9: Send stream reset
                                let _ = event_tx
                                    .send(DeviceEvent::Message(
                                        "Vevor: resetting stream...".to_owned(),
                                    ))
                                    .await;
                                let reset_msg = vevor_usb_protocol::generate_stream_reset_command();
                                let _ = t.write_all(&reset_msg);

                                let _ = event_tx.send(DeviceEvent::JobProgress(100)).await;
                                let _ = event_tx.send(DeviceEvent::JobFinished(true)).await;
                                continue;
                            }

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

                    DeviceCommand::SendRaw(cmd) => {
                        if let Some(ref mut t) = transport {
                            let msg = vevor_usb_protocol::generate_hpgl_message(&cmd);
                            match t.write_all(&msg) {
                                Ok(_) => {
                                    let _ = event_tx
                                        .send(DeviceEvent::LineReceived(format!("→ {cmd}")))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = event_tx
                                        .send(DeviceEvent::Message(format!("SendRaw error: {e}")))
                                        .await;
                                }
                            }
                        } else {
                            let _ = event_tx
                                .send(DeviceEvent::Message(
                                    "Not connected – cannot send command".to_owned(),
                                ))
                                .await;
                        }
                    }
                }
            }
        });
    });

    (cmd_tx, event_rx)
}
