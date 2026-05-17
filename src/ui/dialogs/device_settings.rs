//! Device settings dialog (modal overlay).

use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Color, Element, Length};

use crate::app::Message;
use crate::core::config::{
    DeviceConfig, DeviceConnection, DeviceProfile, SerialFlowControl, SerialParity,
    WifiTargetType,
};
use crate::core::types::DeviceType;

/// The editable state for the device-settings dialog.
#[derive(Debug, Clone, Default)]
pub struct DeviceSettingsState {
    pub connection: DeviceConnection,
    pub port: String,
    pub baud_rate: String,
    pub serial_data_bits: u8,
    pub serial_parity: SerialParity,
    pub serial_stop_bits: u8,
    pub serial_flow_control: SerialFlowControl,
    pub usb_device: String,
    pub wifi_target: String,
    pub wifi_target_type: WifiTargetType,
    pub bluetooth_device: String,
    pub device_type: DeviceType,
    /// Available serial ports (populated at open time).
    pub available_ports: Vec<String>,
    /// Available USB devices discovered from serialport USB metadata.
    pub available_usb_devices: Vec<String>,
    /// Profile name being edited.
    pub profile_name: String,
    /// Work-area width string (mm).
    pub work_area_w: String,
    /// Work-area height string (mm).
    pub work_area_h: String,
    /// All profiles (for the dropdown).
    pub profiles: Vec<DeviceProfile>,
    /// Active profile index.
    pub active_profile: usize,
}

impl DeviceSettingsState {
    pub fn from_config(cfg: &DeviceConfig) -> Self {
        let (available, usb_devices) = scan_connection_devices();
        let active = cfg.active();
        Self {
            connection: active.connection,
            port: active.port.clone(),
            baud_rate: active.baud_rate.to_string(),
            serial_data_bits: active.serial_data_bits,
            serial_parity: active.serial_parity,
            serial_stop_bits: active.serial_stop_bits,
            serial_flow_control: active.serial_flow_control,
            usb_device: active.usb_device.clone(),
            wifi_target: active.wifi_target.clone(),
            wifi_target_type: active.wifi_target_type,
            bluetooth_device: active.bluetooth_device.clone(),
            device_type: active.device_type,
            available_ports: available,
            available_usb_devices: usb_devices,
            profile_name: active.name.clone(),
            work_area_w: format!("{:.1}", active.work_area_w),
            work_area_h: format!("{:.1}", active.work_area_h),
            profiles: cfg.profiles.clone(),
            active_profile: cfg.active_profile,
        }
    }

    pub fn rescan_connection_devices(&mut self) {
        let (available, usb_devices) = scan_connection_devices();
        self.available_ports = available;
        self.available_usb_devices = usb_devices;
    }
}

fn scan_connection_devices() -> (Vec<String>, Vec<String>) {
    let mut available = Vec::new();
    let mut usb_devices = Vec::new();

    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            let name = p.port_name.clone();
            available.push(name.clone());
            if let serialport::SerialPortType::UsbPort(info) = p.port_type {
                let mut parts = vec![name];
                if let Some(product) = info.product {
                    if !product.is_empty() {
                        parts.push(product);
                    }
                }
                if let Some(serial) = info.serial_number {
                    if !serial.is_empty() {
                        parts.push(format!("SN:{serial}"));
                    }
                }
                usb_devices.push(parts.join(" | "));
            }
        }
    }

    usb_devices.extend(windows_usb_printer_queues());
    usb_devices.extend(windows_usbprint_paths());
    usb_devices.sort();
    usb_devices.dedup();

    (available, usb_devices)
}

pub fn device_settings_view(state: &DeviceSettingsState) -> Element<'_, Message> {
    let label = |s: &'static str| {
        text(s)
            .size(13)
            .style(|_: &iced::Theme| text::Style {
                color: Some(Color::WHITE),
            })
            .width(Length::Fixed(100.0))
    };

    // ---- Profile selector row ----
    let profile_names: Vec<String> = state.profiles.iter().map(|p| p.name.clone()).collect();
    let selected_profile = profile_names.get(state.active_profile).cloned();

    let profile_row = row![
        label("Profile"),
        pick_list(profile_names, selected_profile, |chosen| {
            // find index by name
            Message::DeviceProfileSelected(
                state.profiles.iter().position(|p| p.name == chosen).unwrap_or(0),
            )
        })
        .text_size(13)
        .width(Length::Fixed(140.0)),
        button(text("+").size(13))
            .on_press(Message::DeviceProfileNew)
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
        button(text("Delete").size(12))
            .on_press_maybe(if state.profiles.len() > 1 {
                Some(Message::DeviceProfileDelete)
            } else {
                None
            })
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.45, 0.15, 0.15))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    // ---- Profile name ----
    let name_row = row![
        label("Name"),
        text_input("Profile name", &state.profile_name)
            .on_input(Message::DeviceProfileNameChanged)
            .size(13)
            .width(Length::Fixed(200.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // ---- Work area ----
    let work_area_row = row![
        label("Work Area"),
        text_input("400", &state.work_area_w)
            .on_input(|v| {
                Message::DeviceProfileWorkAreaW(v.parse().unwrap_or(0.0))
            })
            .size(13)
            .width(Length::Fixed(70.0)),
        text("×").size(13).style(|_: &iced::Theme| text::Style {
            color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
        }),
        text_input("400", &state.work_area_h)
            .on_input(|v| {
                Message::DeviceProfileWorkAreaH(v.parse().unwrap_or(0.0))
            })
            .size(13)
            .width(Length::Fixed(70.0)),
        text("mm").size(12).style(|_: &iced::Theme| text::Style {
            color: Some(Color::from_rgb(0.6, 0.6, 0.6)),
        }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    // ---- Port ----
    let port_options: Vec<_> = state.available_ports.clone();
    let selected_port = if port_options.contains(&state.port) {
        Some(state.port.clone())
    } else {
        None
    };

    // ---- Baud ----
    let baud_options = vec![
        "9600".to_owned(),
        "19200".to_owned(),
        "38400".to_owned(),
        "57600".to_owned(),
        "115200".to_owned(),
        "230400".to_owned(),
    ];
    let selected_baud = if baud_options.contains(&state.baud_rate) {
        Some(state.baud_rate.clone())
    } else {
        None
    };

    let connection_options: Vec<DeviceConnection> = DeviceConnection::all().to_vec();
    let selected_connection = Some(state.connection);

    let serial_bits_options = vec![
        5u8,
        6u8,
        7u8,
        8u8,
    ];
    let serial_parity_options: Vec<SerialParity> = SerialParity::all().to_vec();
    let serial_stop_bits_options = vec![1u8, 2u8];
    let serial_flow_options: Vec<SerialFlowControl> = SerialFlowControl::all().to_vec();

    let usb_options = state.available_usb_devices.clone();
    let selected_usb = if usb_options.contains(&state.usb_device) {
        Some(state.usb_device.clone())
    } else {
        None
    };

    let wifi_type_options: Vec<WifiTargetType> = WifiTargetType::all().to_vec();
    let selected_wifi_type = Some(state.wifi_target_type);

    let device_type_options: Vec<DeviceType> = DeviceType::all().to_vec();
    let selected_type = Some(state.device_type);

    let mut content = column![
        text("Device Settings")
            .size(16)
            .style(|_: &iced::Theme| text::Style { color: Some(Color::WHITE) }),
        profile_row,
        name_row,
        work_area_row,
        // Connection type
        row![
            label("Connection"),
            pick_list(connection_options, selected_connection, |v| {
                Message::DeviceConnectionChanged(v)
            })
            .text_size(13),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        // Device type
        row![
            label("Device Type"),
            pick_list(device_type_options, selected_type, |v| {
                Message::DeviceTypeChanged(v)
            })
            .text_size(13),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .padding(20);

    match state.connection {
        DeviceConnection::Serial => {
            content = content
                .push(
                    row![
                        label("Serial Port"),
                        pick_list(port_options, selected_port, |v| {
                            Message::DevicePortChanged(v)
                        })
                        .text_size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .push(
                    row![
                        label("Baud Rate"),
                        pick_list(baud_options, selected_baud, |v| {
                            Message::DeviceBaudChanged(v.parse().unwrap_or(115200))
                        })
                        .text_size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .push(
                    row![
                        label("Data Bits"),
                        pick_list(serial_bits_options, Some(state.serial_data_bits), |v| {
                            Message::DeviceSerialDataBitsChanged(v)
                        })
                        .text_size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .push(
                    row![
                        label("Parity"),
                        pick_list(serial_parity_options, Some(state.serial_parity), |v| {
                            Message::DeviceSerialParityChanged(v)
                        })
                        .text_size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .push(
                    row![
                        label("Stop Bits"),
                        pick_list(serial_stop_bits_options, Some(state.serial_stop_bits), |v| {
                            Message::DeviceSerialStopBitsChanged(v)
                        })
                        .text_size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .push(
                    row![
                        label("Flow Ctrl"),
                        pick_list(serial_flow_options, Some(state.serial_flow_control), |v| {
                            Message::DeviceSerialFlowControlChanged(v)
                        })
                        .text_size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
        }
        DeviceConnection::Usb => {
                content = content
                    .push(
                        row![
                            label("USB Device"),
                            pick_list(usb_options, selected_usb, |v| {
                                Message::DeviceUsbDeviceChanged(v)
                            })
                            .text_size(13)
                            .width(Length::Fixed(260.0)),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .push(
                        row![
                            label("USB Target"),
                            text_input("Queue name or path", &state.usb_device)
                                .on_input(Message::DeviceUsbDeviceChanged)
                                .size(13)
                                .width(Length::Fixed(260.0)),
                            button(text("Rescan").size(12))
                                .on_press(Message::DeviceConnectionRescan)
                                .style(|_t, _s| button::Style {
                                    background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
                                    text_color: Color::WHITE,
                                    border: iced::Border { radius: 3.0.into(), ..Default::default() },
                                    ..Default::default()
                                }),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    );
        }
        DeviceConnection::Wifi => {
            content = content
                .push(
                    row![
                        label("Wi-Fi Type"),
                        pick_list(wifi_type_options, selected_wifi_type, |v| {
                            Message::DeviceWifiTargetTypeChanged(v)
                        })
                        .text_size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .push(
                    row![
                        label("Address"),
                        text_input("IP / hostname / URL", &state.wifi_target)
                            .on_input(Message::DeviceWifiTargetChanged)
                            .size(13)
                            .width(Length::Fixed(260.0)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
        }
        DeviceConnection::Bluetooth => {
            content = content.push(
                row![
                    label("Bluetooth"),
                    text_input("Device name or address", &state.bluetooth_device)
                        .on_input(Message::DeviceBluetoothDeviceChanged)
                        .size(13)
                        .width(Length::Fixed(260.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }

    content = content.push(
        row![
            button(text("OK").size(13))
                .on_press(Message::DeviceSettingsOk)
                .style(|_t, _s| button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.45, 0.75))),
                    text_color: Color::WHITE,
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
            button(text("Cancel").size(13))
                .on_press(Message::CloseDialog)
                .style(|_t, _s| button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.3, 0.3, 0.3))),
                    text_color: Color::WHITE,
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
        ]
        .spacing(8),
    );

    modal_container(content.into())
}

fn modal_container(inner: Element<'_, Message>) -> Element<'_, Message> {
    container(
        container(inner)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.16, 0.16, 0.16))),
                border: iced::Border {
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .width(Length::Fixed(520.0)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
        ..Default::default()
    })
    .center(Length::Fill)
    .into()
}

#[cfg(target_os = "windows")]
fn windows_usb_printer_queues() -> Vec<String> {
    use std::process::Command;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-Printer | ForEach-Object { \"Printer Queue: $($_.Name) [$($_.PortName)]\" }",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
fn windows_usbprint_paths() -> Vec<String> {
    use std::process::Command;

    // Prefer direct VID/PID signature from packet capture.
    let by_vid_pid = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Printer | Where-Object { $_.PNPDeviceID -match 'VID_045B&PID_5310' -or $_.PNPDeviceID -match 'VID_045B.*PID_5310' } | ForEach-Object { \"Printer Queue: $($_.Name) [$($_.PortName)]\" }",
        ])
        .output();

    let mut rows = match by_vid_pid {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<String>>(),
        _ => Vec::new(),
    };

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -like 'USBPRINT*' -or $_.InstanceId -match 'VID_045B&PID_5310' -or $_.Class -eq 'SoftwareDevice' -or $_.FriendlyName -match 'PosteK|POSTEK|Q8/200|Vevor' } | ForEach-Object { if ($_.InstanceId -match 'USB\\d{3}') { \"USB Path: \\\\.\\$($Matches[0]) | $($_.FriendlyName)\" } }",
        ])
        .output();

    let mut pnp_rows = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<String>>(),
        _ => Vec::new(),
    };

    // Final fallback: generic USB printer queues to ensure something selectable appears.
    let generic_queues = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Printer | Where-Object { $_.PortName -match '^USB\\d{3}$' -or $_.PNPDeviceID -like 'USBPRINT*' } | ForEach-Object { \"Printer Queue: $($_.Name) [$($_.PortName)]\" }",
        ])
        .output();

    let mut generic_rows = match generic_queues {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<String>>(),
        _ => Vec::new(),
    };

    rows.append(&mut pnp_rows);
    rows.append(&mut generic_rows);
    rows.sort();
    rows.dedup();
    rows
}

#[cfg(not(target_os = "windows"))]
fn windows_usbprint_paths() -> Vec<String> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
fn windows_usb_printer_queues() -> Vec<String> {
    Vec::new()
}
