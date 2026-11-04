//! Bottom hint bar of the main pane: advertises the keyboard and controller
//! shortcut panels.

use iced::widget::{Column, Row, button, container, text};
use iced::{Element, Length};

use crate::message::Message;

use super::focus_style;

/// Height reserved by the hint bar, in logical pixels.
pub(crate) const FOOTER_HEIGHT: f32 = 30.0;

/// Text of the keyboard hint, kept stable for tests.
pub(crate) const KEYBOARD_HINT: &str = "press [?] for Keyboard Shortcuts";
/// Text of the controller hint, kept stable for tests.
pub(crate) const CONTROLLER_HINT: &str = "Press (△/Y) for Controller Shortcuts";

fn hint_button(label: &'static str, message: Message) -> Element<'static, Message> {
    let inner = button(text(label).size(11))
        .padding([2, 6])
        .style(iced::widget::button::text)
        .on_press(message);

    container(inner)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                border: iced::Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: palette.background.strong.color,
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// The bottom hint bar rendered under the main pane content.
pub(crate) fn view() -> Element<'static, Message> {
    let row = Row::new()
        .spacing(8)
        .align_y(iced::Center)
        .push(hint_button(KEYBOARD_HINT, Message::ToggleKeyboardShortcuts))
        .push(hint_button(
            CONTROLLER_HINT,
            Message::ToggleControllerShortcuts,
        ));

    container(
        Column::new()
            .height(Length::Fill)
            .align_x(iced::Center)
            .push(iced::widget::center(row)),
    )
    .width(Length::Fill)
    .height(Length::Fixed(FOOTER_HEIGHT))
    .style(focus_style::footer_style())
    .into()
}
