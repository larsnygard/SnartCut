//! Device panel – connection, position readout, jog controls, job control.

use iced::widget::{button, column, container, pick_list, progress_bar, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};

use crate::app::Message;
use crate::core::config::{Config, DeviceConnection, WifiTargetType};
use crate::core::types::DeviceType;

pub fn device_view<'a>(
    config: &'a Config,
    connected: bool,
    vevor_presence_target: Option<&'a str>,
    vevor_last_status: &'a str,
    position: (f64, f64),
    job_progress: Option<u8>,
    log_messages: &'a [String],
) -> Element<'a, Message> {
    let status_color = if connected {
        Color::from_rgb(0.2, 0.8, 0.2)
    } else {
        Color::from_rgb(0.8, 0.2, 0.2)
    };

    let status_text = if connected { "Connected" } else { "Disconnected" };

    let connect_label = if connected { "Disconnect" } else { "Connect" };

    let profile = config.device.active();
    let conn_summary = match profile.connection {
        DeviceConnection::Serial => {
            let port = if profile.port.trim().is_empty() { "(not set)" } else { profile.port.as_str() };
            format!(
                "Connection: Serial | {} @ {} bps, {}{}{}",
                port,
                profile.baud_rate,
                profile.serial_data_bits,
                match profile.serial_parity {
                    crate::core::config::SerialParity::None => "N",
                    crate::core::config::SerialParity::Even => "E",
                    crate::core::config::SerialParity::Odd => "O",
                },
                profile.serial_stop_bits,
            )
        }
        DeviceConnection::Usb => {
            let dev = if profile.usb_device.trim().is_empty() {
                "(not set)"
            } else {
                profile.usb_device.as_str()
            };
            if profile.device_type == crate::core::types::DeviceType::VevorSmart1 {
                format!("Connection: USB | {dev} | Vevor auto-detect on Connect")
            } else {
                format!("Connection: USB | {dev}")
            }
        }
        DeviceConnection::Wifi => {
            let ty = match profile.wifi_target_type {
                WifiTargetType::IpAddress => "IP",
                WifiTargetType::Hostname => "Hostname",
                WifiTargetType::Url => "URL",
            };
            let target = if profile.wifi_target.trim().is_empty() {
                "(not set)"
            } else {
                profile.wifi_target.as_str()
            };
            format!("Connection: Wi-Fi | {ty}: {target}")
        }
        DeviceConnection::Bluetooth => {
            let dev = if profile.bluetooth_device.trim().is_empty() {
                "(not set)"
            } else {
                profile.bluetooth_device.as_str()
            };
            format!("Connection: Bluetooth | {dev}")
        }
    };

    let vevor_status_row = if profile.device_type == crate::core::types::DeviceType::VevorSmart1
        && profile.connection == DeviceConnection::Usb
    {
        let (txt, color) = if connected {
            ("Vevor Status: Connected".to_owned(), Color::from_rgb(0.2, 0.8, 0.2))
        } else if let Some(target) = vevor_presence_target {
            (
                format!("Vevor Status: Detected ({target})"),
                Color::from_rgb(0.85, 0.75, 0.25),
            )
        } else {
            (
                "Vevor Status: Not detected".to_owned(),
                Color::from_rgb(0.8, 0.3, 0.3),
            )
        };
        Some(row![
            text(txt)
                .size(11)
                .style(move |_: &iced::Theme| text::Style {
                    color: Some(color),
                }),
        ])
    } else {
        None
    };

    let _label = |s: &'static str| -> iced::widget::Text<'static> {
        text(s)
            .size(12)
            .style(|_: &iced::Theme| text::Style {
                color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
            })
            .width(Length::Fixed(50.0))
    };

    // Profile selector row
    let profile_names: Vec<String> =
        config.device.profiles.iter().map(|p| p.name.clone()).collect();
    let active_profile_name = profile_names
        .get(config.device.active_profile)
        .cloned();
    let profile_row = row![
        text("Profile:").size(12).style(|_: &iced::Theme| text::Style {
            color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
        }),
        pick_list(profile_names, active_profile_name, |chosen: String| {
            Message::DeviceProfileSelected(
                config.device.profiles.iter().position(|p| p.name == chosen).unwrap_or(0),
            )
        })
        .text_size(12)
        .width(Length::Fixed(120.0)),
        button(text("Settings…").size(12))
            .on_press(Message::OpenDeviceSettings)
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    // Connection row
    let conn_row = row![
        text(status_text)
            .size(13)
            .style(move |_: &iced::Theme| text::Style {
                color: Some(status_color),
            }),
        button(text(connect_label).size(12))
            .on_press(if connected {
                Message::DisconnectDevice
            } else {
                Message::ConnectDevice
            })
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let conn_summary_row = row![
        text(conn_summary)
            .size(11)
            .style(|_: &iced::Theme| text::Style {
                color: Some(Color::from_rgb(0.75, 0.75, 0.75)),
            }),
    ];

    // Position and jog
    let pos_row = row![
        text(format!("X: {:.3}  Y: {:.3}", position.0, position.1))
            .size(12)
            .style(|_: &iced::Theme| text::Style {
                color: Some(Color::WHITE),
            }),
    ];

    let jog_row = row![
        button(text("←").size(14))
            .on_press_maybe(if connected {
                Some(Message::JogDevice(-10.0, 0.0))
            } else {
                None
            }),
        button(text("↑").size(14))
            .on_press_maybe(if connected {
                Some(Message::JogDevice(0.0, -10.0))
            } else {
                None
            }),
        button(text("↓").size(14))
            .on_press_maybe(if connected {
                Some(Message::JogDevice(0.0, 10.0))
            } else {
                None
            }),
        button(text("→").size(14))
            .on_press_maybe(if connected {
                Some(Message::JogDevice(10.0, 0.0))
            } else {
                None
            }),
        button(text("Home").size(12))
            .on_press_maybe(if connected {
                Some(Message::HomeDevice)
            } else {
                None
            }),
    ]
    .spacing(4);

    // Job controls
    let job_row = row![
        button(text("▶ Run").size(12))
            .on_press_maybe(if connected { Some(Message::SendJob) } else { None })
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.55, 0.1))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
        button(text("⏸ Pause").size(12))
            .on_press_maybe(if connected { Some(Message::PauseJob) } else { None })
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.55, 0.45, 0.0))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
        button(text("■ Stop").size(12))
            .on_press_maybe(if connected { Some(Message::CancelJob) } else { None })
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.6, 0.1, 0.1))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    ]
    .spacing(4);

    // Progress bar
    let progress = if let Some(pct) = job_progress {
        Some(
            progress_bar(0.0..=100.0, pct as f32)
                .height(Length::Fixed(10.0))
                .style(|_| progress_bar::Style {
                    background: iced::Background::Color(Color::from_rgb(0.2, 0.2, 0.2)),
                    bar: iced::Background::Color(Color::from_rgb(0.1, 0.6, 0.1)),
                    border: iced::Border::default(),
                }),
        )
    } else {
        None
    };

    // Log
    let log_col = log_messages.iter().rev().take(20).fold(
        column![].spacing(1),
        |col, msg| {
            col.push(
                text(msg)
                    .size(10)
                    .style(|_: &iced::Theme| text::Style {
                        color: Some(Color::from_rgb(0.65, 0.65, 0.65)),
                    }),
            )
        },
    );

    let mut main_col = column![
        profile_row,
        conn_row,
        conn_summary_row,
        pos_row,
        jog_row,
        job_row,
    ]
    .spacing(4)
    .padding(4);

    if let Some(vs) = vevor_status_row {
        main_col = main_col.push(vs);
    }

    // Modular device-specific controls
    if profile.device_type == DeviceType::VevorSmart1 {
        main_col = main_col.push(
            crate::device::vevor::controls_view(connected, vevor_last_status)
        );
    }

    if let Some(bar) = progress {
        main_col = main_col.push(bar);
    }

    main_col = main_col.push(scrollable(log_col).height(Length::Fixed(60.0)));

    container(main_col)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.10, 0.10, 0.10))),
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
}
