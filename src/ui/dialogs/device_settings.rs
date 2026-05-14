//! Device settings dialog (modal overlay).

use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Color, Element, Length};

use crate::app::Message;
use crate::core::config::{DeviceConfig, DeviceProfile};
use crate::core::types::DeviceType;

/// The editable state for the device-settings dialog.
#[derive(Debug, Clone, Default)]
pub struct DeviceSettingsState {
    pub port: String,
    pub baud_rate: String,
    pub device_type: DeviceType,
    /// Available serial ports (populated at open time).
    pub available_ports: Vec<String>,
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
        let mut available = Vec::new();
        if let Ok(ports) = serialport::available_ports() {
            available = ports.into_iter().map(|p| p.port_name).collect();
        }
        let active = cfg.active();
        Self {
            port: active.port.clone(),
            baud_rate: active.baud_rate.to_string(),
            device_type: active.device_type,
            available_ports: available,
            profile_name: active.name.clone(),
            work_area_w: format!("{:.1}", active.work_area_w),
            work_area_h: format!("{:.1}", active.work_area_h),
            profiles: cfg.profiles.clone(),
            active_profile: cfg.active_profile,
        }
    }
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

    let device_type_options: Vec<DeviceType> = DeviceType::all().to_vec();
    let selected_type = Some(state.device_type);

    let content = column![
        text("Device Settings")
            .size(16)
            .style(|_: &iced::Theme| text::Style { color: Some(Color::WHITE) }),
        profile_row,
        name_row,
        work_area_row,
        // Port
        row![
            label("Serial Port"),
            pick_list(port_options, selected_port, |v| {
                Message::DevicePortChanged(v)
            })
            .text_size(13),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        // Baud rate
        row![
            label("Baud Rate"),
            pick_list(baud_options, selected_baud, |v| {
                Message::DeviceBaudChanged(v.parse().unwrap_or(115200))
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
        // Buttons
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
    ]
    .spacing(12)
    .padding(20);

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
            .width(Length::Fixed(420.0)),
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
