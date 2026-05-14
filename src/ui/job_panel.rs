//! Job panel – workspace settings and G-code export.

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length};

use crate::app::Message;
use crate::job::settings::JobSettings;

pub fn job_view(job: &JobSettings) -> Element<'_, Message> {
    let label = |s: &'static str| {
        text(s)
            .size(12)
            .style(|_: &iced::Theme| text::Style {
                color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
            })
            .width(Length::Fixed(90.0))
    };

    let col = column![
        text("Job Settings")
            .size(14)
            .style(|_: &iced::Theme| text::Style { color: Some(Color::WHITE) }),
        // Workspace width
        row![
            label("Width mm"),
            text_input("400", &format!("{:.1}", job.workspace.width_mm))
                .on_input(|v| {
                    let val = v.parse().unwrap_or(job.workspace.width_mm);
                    Message::WorkspaceWidthChanged(val)
                })
                .size(12),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        // Workspace height
        row![
            label("Height mm"),
            text_input("400", &format!("{:.1}", job.workspace.height_mm))
                .on_input(|v| {
                    let val = v.parse().unwrap_or(job.workspace.height_mm);
                    Message::WorkspaceHeightChanged(val)
                })
                .size(12),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        // Material
        row![
            label("Material"),
            text_input("", &job.material)
                .on_input(Message::MaterialNoteChanged)
                .size(12),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        // Notes
        text("Notes")
            .size(12)
            .style(|_: &iced::Theme| text::Style {
                color: Some(Color::from_rgb(0.7, 0.7, 0.7)),
            }),
        text_input("Operator notes…", &job.notes)
            .on_input(Message::JobNotesChanged)
            .size(12),
        // Export button
        button(text("Export G-code…").size(13))
            .on_press(Message::ExportGcode)
            .style(|_t, _s| button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.45, 0.75))),
                text_color: Color::WHITE,
                border: iced::Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            })
            .width(Length::Fill),
    ]
    .spacing(6)
    .padding(6);

    container(col)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
