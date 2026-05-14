//! Material library dialog (modal overlay).

use iced::widget::{button, column, container, scrollable, text};
use iced::{Color, Element, Length};

use crate::app::Message;
use crate::core::types::CutSettings;
use crate::job::settings::material_library;

pub fn material_library_view(selected: Option<&str>) -> Element<'_, Message> {
    let lib = material_library();
    let mut names: Vec<&str> = lib.keys().copied().collect();
    names.sort_unstable();

    let list = names.iter().fold(column![].spacing(2), |col, &name| {
        let is_sel = selected == Some(name);
        col.push(
            button(
                text(name).size(13).style(move |_: &iced::Theme| text::Style {
                    color: Some(Color::WHITE),
                }),
            )
            .on_press(Message::MaterialPresetSelected(name.to_owned()))
            .style(move |_t, _s| button::Style {
                background: Some(iced::Background::Color(if is_sel {
                    Color::from_rgba(0.0, 0.47, 0.83, 0.4)
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.05)
                })),
                text_color: Color::WHITE,
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            })
            .width(Length::Fill),
        )
    });

    let content = column![
        text("Material Library")
            .size(16)
            .style(|_: &iced::Theme| text::Style { color: Some(Color::WHITE) }),
        scrollable(list).height(Length::Fixed(320.0)),
        // Buttons
        iced::widget::row![
            button(text("Apply").size(13))
                .on_press_maybe(
                    selected.map(|n| Message::MaterialPresetApply(n.to_owned()))
                )
                .style(|_t, _s| button::Style {
                    background: Some(iced::Background::Color(
                        Color::from_rgb(0.15, 0.45, 0.75)
                    )),
                    text_color: Color::WHITE,
                    border: iced::Border { radius: 4.0.into(), ..Default::default() },
                    ..Default::default()
                }),
            button(text("Close").size(13))
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
    .spacing(10)
    .padding(20);

    container(
        container(content)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.16, 0.16, 0.16))),
                border: iced::Border {
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .width(Length::Fixed(360.0)),
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
