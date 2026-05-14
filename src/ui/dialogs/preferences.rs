//! Preferences dialog – visual and general application settings.

use iced::widget::{
    button, checkbox, column, container, horizontal_rule, row, scrollable,
    slider, text, text_input,
};
use iced::{Alignment, Element, Length, Padding};

use crate::app::Message;
use crate::core::config::VisualConfig;

/// Render the Preferences dialog content.
///
/// The dialog is wrapped in the same modal overlay used by other dialogs.
pub fn preferences_view(visual: &VisualConfig) -> Element<'static, Message> {
    let title = text("Preferences")
        .size(18)
        .style(|_: &_| text::Style { color: Some(iced::Color::WHITE) });

    let content = column![
        section_header("Canvas"),
        color_row(
            "Background colour",
            "Outer dark area surrounding the work area",
            &visual.canvas_bg,
            |s| Message::PrefCanvasBgChanged(s),
        ),
        color_row(
            "Work area colour",
            "Fill colour of the laser work area",
            &visual.workspace_bg,
            |s| Message::PrefWorkspaceBgChanged(s),
        ),
        color_row(
            "Grid colour",
            "Grid line colour (opacity set separately)",
            &visual.grid_color,
            |s| Message::PrefGridColorChanged(s),
        ),
        opacity_row(
            "Grid opacity",
            visual.grid_opacity,
            |v| Message::PrefGridOpacityChanged(v),
        ),
        iced::widget::Space::with_height(8),
        section_header("Shapes"),
        color_row(
            "Selection colour",
            "Highlight colour for selected shapes",
            &visual.selection_color,
            |s| Message::PrefSelectionColorChanged(s),
        ),
        color_row(
            "Preview colour",
            "Colour of the live shape preview while drawing",
            &visual.preview_color,
            |s| Message::PrefPreviewColorChanged(s),
        ),
        stroke_width_row(
            "Shape stroke width (px)",
            visual.shape_stroke_px,
            |v| Message::PrefShapeStrokeChanged(v),
        ),
        antialiasing_row(visual.antialiasing),
    ]
    .spacing(2)
    .padding(Padding::from([0, 4]));

    let footer = row![
        iced::widget::Space::with_width(Length::Fill),
        button(text("Cancel").size(14))
            .on_press(Message::CloseModal)
            .style(button::secondary),
        button(text("Save").size(14))
            .on_press(Message::PrefSave)
            .style(button::primary),
    ]
    .spacing(8)
    .padding(iced::Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 });

    let inner = column![
        title,
        horizontal_rule(1),
        scrollable(content).height(Length::Fixed(380.0)),
        footer,
    ]
    .spacing(12)
    .padding(20)
    .width(Length::Fixed(480.0));

    container(inner)
        .style(|_: &_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.13, 0.13, 0.15))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.35, 0.35, 0.4),
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: Some(iced::Color::WHITE),
            shadow: iced::Shadow {
                color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
        })
        .into()
}

// ────────────────────────────────────────────────────────────────────
// Private helpers
// ────────────────────────────────────────────────────────────────────

fn section_header(label: &str) -> Element<'static, Message> {
    let label = label.to_owned();
    column![
        text(label).size(13).style(|_: &_| text::Style {
            color: Some(iced::Color::from_rgb(0.55, 0.75, 1.0)),
        }),
        horizontal_rule(1),
    ]
    .spacing(2)
    .padding(Padding::from([4, 0]))
    .into()
}

fn color_row<F>(
    label: &str,
    hint: &str,
    value: &str,
    on_change: F,
) -> Element<'static, Message>
where
    F: Fn(String) -> Message + 'static,
{
    let label_text = label.to_owned();
    let hint_text = hint.to_owned();
    let current = value.to_owned();

    let swatch_color = crate::ui::canvas_widget::hex_to_color(&current)
        .unwrap_or(iced::Color::from_rgb(0.5, 0.5, 0.5));

    let swatch = container(iced::widget::Space::new(24, 24))
        .style(move |_: &_| container::Style {
            background: Some(iced::Background::Color(swatch_color)),
            border: iced::Border {
                color: iced::Color::from_rgb(0.5, 0.5, 0.5),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        });

    let input = text_input("#rrggbb", &current)
        .on_input(on_change)
        .width(Length::Fixed(90.0))
        .size(13);

    row![
        column![
            text(label_text).size(13),
            text(hint_text).size(11).style(|_: &_| text::Style {
                color: Some(iced::Color::from_rgb(0.55, 0.55, 0.55)),
            }),
        ]
        .spacing(2)
        .width(Length::Fill),
        swatch,
        iced::widget::Space::with_width(6),
        input,
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .padding(Padding::from([4, 0]))
    .into()
}

fn opacity_row<F>(label: &str, value: f32, on_change: F) -> Element<'static, Message>
where
    F: Fn(f32) -> Message + 'static,
{
    let label_text = label.to_owned();
    let pct = (value * 100.0).round() as u8;

    row![
        text(label_text).size(13).width(Length::Fill),
        slider(0.0f32..=1.0, value, on_change).step(0.01).width(Length::Fixed(160.0)),
        text(format!("{pct}%")).size(13).width(Length::Fixed(36.0)),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .padding(Padding::from([4, 0]))
    .into()
}

fn stroke_width_row<F>(label: &str, value: f32, on_change: F) -> Element<'static, Message>
where
    F: Fn(f32) -> Message + 'static,
{
    let label_text = label.to_owned();

    row![
        text(label_text).size(13).width(Length::Fill),
        slider(0.5f32..=8.0, value, on_change).step(0.5).width(Length::Fixed(160.0)),
        text(format!("{value:.1} px")).size(13).width(Length::Fixed(48.0)),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .padding(Padding::from([4, 0]))
    .into()
}

fn antialiasing_row(value: bool) -> Element<'static, Message> {
    row![
        text("Antialiasing").size(13).width(Length::Fill),
        checkbox("", value).on_toggle(Message::PrefAntialiasingChanged),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .padding(Padding::from([4, 0]))
    .into()
}
