//! Floating toast notification, anchored to the bottom-right corner.
//!
//! The toast is purely presentational: it contains no interactive widgets, so
//! it never requests keyboard focus and the text editor keeps receiving input
//! while the notification is visible.

use iced::theme::Palette;
use iced::widget::{column, container, text};
use iced::{Alignment, Background, Border, Color, Length, Shadow, Vector};

use crate::types::{blend, Message};

/// Convenience alias for this component's widgets.
type Element<'a> = iced::Element<'a, Message>;

/// Layers an optional toast on top of `base`, in the bottom-right corner.
///
/// The `Stack` is always created, even when there is no toast, so that the
/// widget tree (and therefore the text editor's focus) stays stable while the
/// notification appears and disappears.
pub fn overlay<'a>(base: Element<'a>, notice: Option<(&'a str, Palette)>) -> Element<'a> {
    let floating = notice.map(|(message, palette)| {
        let layer: Element<'a> = container(view(message, palette))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .align_x(Alignment::End)
            .align_y(Alignment::End)
            .into();

        layer
    });

    iced::widget::stack([base])
        .push_maybe(floating)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The notification card itself.
fn view<'a>(message: &'a str, palette: Palette) -> Element<'a> {
    container(
        column![
            text("Session notice")
                .size(12)
                .color(blend(palette.background, palette.primary, 0.75)),
            text(message).size(15).color(palette.text),
        ]
        .spacing(4),
    )
    .padding([14, 18])
    .max_width(340.0)
    .style(move |_theme| container::Style {
        text_color: Some(palette.text),
        background: Some(Background::from(blend(
            palette.background,
            palette.text,
            0.10,
        ))),
        border: Border {
            color: blend(palette.background, palette.primary, 0.55),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 14.0,
        },
    })
    .into()
}
