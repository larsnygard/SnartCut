//! Preferences dialog – visual settings and key/mouse bindings.

use iced::widget::{
    button, checkbox, column, container, horizontal_rule, pick_list, row,
    scrollable, slider, text, text_input,
};
use iced::{Alignment, Color, Element, Length, Padding};

use crate::app::Message;
use crate::core::config::{BindingId, KeyBindings, MouseBindings, ScrollAction, VisualConfig};

// ---------------------------------------------------------------------------
// State types (re-exported to app.rs)
// ---------------------------------------------------------------------------

/// Which tab is shown in the Preferences dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrefTab {
    #[default]
    Visual,
    Bindings,
}

/// Runtime state for the Preferences modal.
#[derive(Debug, Clone, Default)]
pub struct PreferencesState {
    pub tab: PrefTab,
    /// When `Some(id)`, the dialog is waiting for the next key press to set
    /// that binding.
    pub capturing: Option<BindingId>,
}

// ---------------------------------------------------------------------------
// Main view entry point
// ---------------------------------------------------------------------------

pub fn preferences_view(
    tab: PrefTab,
    capturing: Option<BindingId>,
    visual: &VisualConfig,
    bindings: &KeyBindings,
    mouse_bindings: &MouseBindings,
) -> Element<'static, Message> {
    // ---- Tab bar ----
    let tab_bar = row![
        tab_button("Visual",   tab == PrefTab::Visual,   Message::PrefTabSelected(PrefTab::Visual)),
        tab_button("Bindings", tab == PrefTab::Bindings, Message::PrefTabSelected(PrefTab::Bindings)),
    ]
    .spacing(2);

    // ---- Tab content ----
    let content: Element<'static, Message> = match tab {
        PrefTab::Visual   => visual_tab(visual),
        PrefTab::Bindings => bindings_tab(capturing, bindings, mouse_bindings),
    };

    // ---- Footer ----
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
    .padding(Padding { top: 8.0, right: 0.0, bottom: 0.0, left: 0.0 });

    let inner = column![
        text("Preferences")
            .size(18)
            .style(|_: &_| text::Style { color: Some(Color::WHITE) }),
        horizontal_rule(1),
        tab_bar,
        scrollable(content).height(Length::Fixed(360.0)),
        footer,
    ]
    .spacing(10)
    .padding(20)
    .width(Length::Fixed(500.0));

    container(inner)
        .style(|_: &_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.13, 0.13, 0.15))),
            border: iced::Border {
                color: Color::from_rgb(0.35, 0.35, 0.4),
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: Some(Color::WHITE),
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
        })
        .into()
}

// ---------------------------------------------------------------------------
// Tab: Visual
// ---------------------------------------------------------------------------

fn visual_tab(visual: &VisualConfig) -> Element<'static, Message> {
    column![
        section_header("Canvas"),
        color_row(
            "Background colour",
            "Outer dark area surrounding the work area",
            &visual.canvas_bg,
            Message::PrefCanvasBgChanged,
        ),
        color_row(
            "Work area colour",
            "Fill colour of the work area",
            &visual.workspace_bg,
            Message::PrefWorkspaceBgChanged,
        ),
        color_row(
            "Grid colour",
            "Grid line colour (opacity set separately)",
            &visual.grid_color,
            Message::PrefGridColorChanged,
        ),
        opacity_row("Grid opacity", visual.grid_opacity, Message::PrefGridOpacityChanged),
        iced::widget::Space::with_height(8),
        section_header("Shapes"),
        color_row(
            "Selection colour",
            "Highlight colour for selected shapes",
            &visual.selection_color,
            Message::PrefSelectionColorChanged,
        ),
        color_row(
            "Preview colour",
            "Colour of the live shape preview while drawing",
            &visual.preview_color,
            Message::PrefPreviewColorChanged,
        ),
        stroke_width_row("Shape stroke width (px)", visual.shape_stroke_px, Message::PrefShapeStrokeChanged),
        antialiasing_row(visual.antialiasing),
    ]
    .spacing(2)
    .padding(Padding::from([4, 0]))
    .into()
}

// ---------------------------------------------------------------------------
// Tab: Bindings
// ---------------------------------------------------------------------------

fn bindings_tab(
    capturing: Option<BindingId>,
    bindings: &KeyBindings,
    mouse_bindings: &MouseBindings,
) -> Element<'static, Message> {
    let mut col = column![section_header("Keyboard")].spacing(2);

    for &id in BindingId::all() {
        col = col.push(binding_row(id, bindings.get(id), capturing));
    }

    col = col.push(iced::widget::Space::with_height(8));
    col = col.push(section_header("Mouse"));
    col = col.push(mouse_scroll_row(mouse_bindings.scroll));
    col = col.push(info_row("Middle-mouse drag", "Pan (fixed)"));

    col.padding(Padding::from([4, 0])).into()
}

fn binding_row(id: BindingId, current: &str, capturing: Option<BindingId>) -> Element<'static, Message> {
    let is_capturing = capturing == Some(id);
    let label_str    = id.label();
    let current_str  = current.to_owned();

    let key_display: Element<'static, Message> = if is_capturing {
        text("Press a key…")
            .size(13)
            .style(|_: &_| text::Style { color: Some(Color::from_rgb(1.0, 0.75, 0.2)) })
            .width(Length::Fixed(110.0))
            .into()
    } else {
        let display = if current_str.is_empty() { "—".to_owned() } else { current_str };
        container(
            text(display).size(12).style(|_: &_| text::Style { color: Some(Color::WHITE) }),
        )
        .style(|_: &_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.2, 0.2, 0.22))),
            border: iced::Border {
                color: Color::from_rgb(0.35, 0.35, 0.4),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .padding(Padding::from([2, 8]))
        .width(Length::Fixed(110.0))
        .into()
    };

    let rebind_btn = button(
        text(if is_capturing { "Cancel" } else { "Rebind" }).size(12),
    )
    .on_press(if is_capturing {
        // Tab-selected with same tab clears capturing without changing tabs
        Message::PrefTabSelected(PrefTab::Bindings)
    } else {
        Message::PrefBeginRebind(id)
    })
    .style(move |_t, _s| {
        let bg = if is_capturing { Color::from_rgb(0.5, 0.3, 0.05) } else { Color::from_rgb(0.25, 0.25, 0.28) };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::WHITE,
            border: iced::Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        }
    });

    let clear_btn = button(text("×").size(13))
        .on_press(Message::PrefClearBinding(id))
        .style(|_t, _s| button::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.35, 0.18, 0.18))),
            text_color: Color::WHITE,
            border: iced::Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        });

    row![
        text(label_str)
            .size(13)
            .style(|_: &_| text::Style { color: Some(Color::from_rgb(0.85, 0.85, 0.85)) })
            .width(Length::Fixed(160.0)),
        key_display,
        rebind_btn,
        clear_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding(Padding::from([3, 0]))
    .into()
}

fn mouse_scroll_row(current: ScrollAction) -> Element<'static, Message> {
    let options = vec![ScrollAction::Zoom, ScrollAction::PanVertical];
    row![
        text("Scroll wheel")
            .size(13)
            .style(|_: &_| text::Style { color: Some(Color::from_rgb(0.85, 0.85, 0.85)) })
            .width(Length::Fixed(160.0)),
        pick_list(options, Some(current), Message::PrefScrollChanged).text_size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding(Padding::from([3, 0]))
    .into()
}

fn info_row(label: &str, value: &str) -> Element<'static, Message> {
    let (l, v) = (label.to_owned(), value.to_owned());
    row![
        text(l)
            .size(13)
            .style(|_: &_| text::Style { color: Some(Color::from_rgb(0.85, 0.85, 0.85)) })
            .width(Length::Fixed(160.0)),
        text(v).size(13).style(|_: &_| text::Style { color: Some(Color::from_rgb(0.5, 0.5, 0.5)) }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding(Padding::from([3, 0]))
    .into()
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn tab_button(label: &str, active: bool, msg: Message) -> Element<'static, Message> {
    let label = label.to_owned();
    let bg = if active { Color::from_rgb(0.18, 0.38, 0.65) } else { Color::from_rgb(0.22, 0.22, 0.25) };
    button(text(label).size(13))
        .on_press(msg)
        .style(move |_t, _s| button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: Color::WHITE,
            border: iced::Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

fn section_header(label: &str) -> Element<'static, Message> {
    let label = label.to_owned();
    column![
        text(label).size(13).style(|_: &_| text::Style {
            color: Some(Color::from_rgb(0.55, 0.75, 1.0)),
        }),
        horizontal_rule(1),
    ]
    .spacing(4)
    .padding(Padding::from([4, 0]))
    .into()
}

fn color_row<F>(label: &str, hint: &str, value: &str, on_change: F) -> Element<'static, Message>
where
    F: Fn(String) -> Message + 'static,
{
    let label_text = label.to_owned();
    let hint_text  = hint.to_owned();
    let current    = value.to_owned();

    let swatch_color = crate::ui::canvas_widget::hex_to_color(&current)
        .unwrap_or(Color::from_rgb(0.5, 0.5, 0.5));

    let swatch = container(iced::widget::Space::new(24, 24))
        .style(move |_: &_| container::Style {
            background: Some(iced::Background::Color(swatch_color)),
            border: iced::Border {
                color: Color::from_rgb(0.5, 0.5, 0.5),
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
                color: Some(Color::from_rgb(0.55, 0.55, 0.55)),
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


