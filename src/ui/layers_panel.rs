//! Layers panel – sidebar showing the cut-layer list.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Alignment, Color, Element, Length};

use crate::app::Message;
use crate::core::types::CutSettings;
use crate::job::layer::LayerList;
use crate::ui::canvas_widget::hex_to_color;

pub fn layers_view<'a>(
    layers: &'a LayerList,
    selected_idx: Option<usize>,
) -> Element<'a, Message> {
    let mut col = Column::new()
        .spacing(2)
        .push(
            text("Cut Layers")
                .size(14)
                .style(|_theme| text::Style { color: Some(Color::WHITE) }),
        );

    // Layer list
    let layer_list: Column<Message> = layers.iter().enumerate().fold(
        Column::new().spacing(1),
        |col, (i, layer)| {
            let is_sel = selected_idx == Some(i);
            let color =
                hex_to_color(layer.color()).unwrap_or(Color::from_rgb(1.0, 0.0, 0.0));

            let label = row![
                // Colour swatch
                container(text(" "))
                    .width(Length::Fixed(12.0))
                    .height(Length::Fixed(16.0))
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(color)),
                        ..Default::default()
                    }),
                text(layer.name())
                    .size(13)
                    .style(move |_| text::Style {
                        color: Some(if layer.enabled() {
                            Color::WHITE
                        } else {
                            Color::from_rgb(0.5, 0.5, 0.5)
                        }),
                    }),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            let row_btn = button(label)
                .on_press(Message::SelectLayer(i))
                .style(move |_theme, _status| button::Style {
                    background: Some(iced::Background::Color(if is_sel {
                        Color::from_rgba(0.0, 0.47, 0.83, 0.4)
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.05)
                    })),
                    text_color: Color::WHITE,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .width(Length::Fill);

            col.push(row_btn)
        },
    );

    col = col.push(scrollable(layer_list).height(Length::Fixed(200.0)));

    // Add / remove buttons
    let btn_row = row![
        button(text("+").size(16))
            .on_press(Message::AddLayer)
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.2, 0.6, 0.2))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
        button(text("−").size(16))
            .on_press_maybe(selected_idx.map(Message::RemoveLayer))
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.6, 0.2, 0.2))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
        button(text("Preset…").size(12))
            .on_press(Message::OpenMaterialLibrary)
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }),
    ]
    .spacing(4);

    col = col.push(btn_row);

    // Settings for the selected layer
    if let Some(idx) = selected_idx {
        if let Some(layer) = layers.get(idx) {
            col = col.push(layer_settings_form(idx, &layer.settings));
        }
    }

    container(col.spacing(6).padding(6))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn layer_settings_form(idx: usize, s: &CutSettings) -> Element<'_, Message> {
    let label_style = |_: &iced::Theme| text::Style {
        color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
    };

    column![
        text("Layer Settings").size(12).style(label_style),
        // Name
        row![
            text("Name").size(12).width(Length::Fixed(60.0)).style(label_style),
            text_input("Layer name", &s.name)
                .on_input(move |v| Message::LayerNameChanged(idx, v))
                .size(12),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        // Speed
        row![
            text("Speed mm/s").size(12).width(Length::Fixed(80.0)).style(label_style),
            text_input("100", &format!("{:.1}", s.speed_mm_s))
                .on_input(move |v| {
                    let val = v.parse().unwrap_or(s.speed_mm_s);
                    Message::LayerSpeedChanged(idx, val)
                })
                .size(12),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        // Power
        row![
            text("Power %").size(12).width(Length::Fixed(80.0)).style(label_style),
            text_input("50", &format!("{:.1}", s.power_pct))
                .on_input(move |v| {
                    let val = v.parse().unwrap_or(s.power_pct);
                    Message::LayerPowerChanged(idx, val)
                })
                .size(12),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        // Passes
        row![
            text("Passes").size(12).width(Length::Fixed(80.0)).style(label_style),
            text_input("1", &s.passes.to_string())
                .on_input(move |v| {
                    let val = v.parse().unwrap_or(s.passes);
                    Message::LayerPassesChanged(idx, val)
                })
                .size(12),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        // Enable toggle
        button(
            text(if s.enabled { "✓ Enabled" } else { "✗ Disabled" })
                .size(12)
                .style(move |_| text::Style {
                    color: Some(if s.enabled {
                        Color::from_rgb(0.2, 0.8, 0.2)
                    } else {
                        Color::from_rgb(0.6, 0.2, 0.2)
                    }),
                }),
        )
        .on_press(Message::LayerEnabledToggled(idx, !s.enabled))
        .style(|_t, _s| button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            border: iced::Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        })
        .width(Length::Fill),
    ]
    .spacing(4)
    .padding(4)
    .into()
}
