//! Keyboard and controller shortcut help panels.
//!
//! iced 0.14 has no modal/stack widget, so the panels are rendered as a
//! full-window replacement layer while open.

use iced::widget::{Column, Row, button, container, scrollable, text};
use iced::{Element, Length};

use crate::message::Message;

use super::focus_style;

/// One row of the shortcut tables: key label + what it does.
const KEYBOARD_SHORTCUTS: &[(&str, &str)] = &[
    ("TAB", "Focus the next pane (Sidebar → Main → Actions)"),
    ("SHIFT+TAB", "Focus the previous pane"),
    ("↑ / ↓", "Move between options in the focused pane"),
    (
        "← / →",
        "Move between horizontal options, e.g. action buttons",
    ),
    (
        "",
        "(cycles panes when there is no option in that direction)",
    ),
    ("SPACE", "Select / activate the focused option"),
    (
        "ENTER",
        "Perform the view's main action (Continue, Download, …)",
    ),
    ("?", "Show this keyboard shortcut list"),
    ("ESC", "Close an open shortcut list"),
];

const CONTROLLER_SHORTCUTS: &[(&str, &str)] = &[
    ("RB / RT", "Focus the next pane"),
    ("LB / LT", "Focus the previous pane"),
    (
        "D-PAD / LEFT STICK",
        "Move between options in the focused pane",
    ),
    (
        "D-PAD ← / →",
        "Move between horizontal options, e.g. action buttons",
    ),
    (
        "",
        "(cycles panes when there is no option in that direction)",
    ),
    ("(A) / (X)", "Select / activate the focused option"),
    (
        "START",
        "Perform the view's main action (Continue, Download, …)",
    ),
    ("(△/Y)", "Show this controller shortcut list"),
    ("(B) / (○)", "Close an open shortcut list (same as Esc)"),
];

/// Renders the keyboard shortcut panel.
pub(crate) fn keyboard_view() -> Element<'static, Message> {
    panel(
        "Keyboard Shortcuts",
        KEYBOARD_SHORTCUTS,
        Message::CloseShortcuts,
    )
}

/// Renders the controller shortcut panel.
pub(crate) fn controller_view() -> Element<'static, Message> {
    panel(
        "Controller Shortcuts",
        CONTROLLER_SHORTCUTS,
        Message::CloseShortcuts,
    )
}

fn panel(
    title: &'static str,
    shortcuts: &'static [(&'static str, &'static str)],
    close: Message,
) -> Element<'static, Message> {
    let mut rows = Column::new().spacing(6).push(text(title).size(18));

    rows = rows.push(focus_style::panel_rule());

    for (key, description) in shortcuts {
        let mut row = Row::new().spacing(10).align_y(iced::Center);
        if key.is_empty() {
            row = row.push(container(text("").size(11)).width(Length::Fixed(170.0)));
        } else {
            row = row.push(container(focus_style::keycap(key)).width(Length::Fixed(170.0)));
        }
        row = row.push(text(*description).size(12));
        rows = rows.push(row);
    }

    rows = rows.push(focus_style::panel_rule());

    rows = rows.push(
        button(text("Close (Esc)").size(13))
            .padding(8)
            .on_press(close),
    );

    let content: Element<'static, Message> = scrollable(rows.spacing(10).padding(20)).into();

    // Dark scrim over the whole window, with the panel centered inside it.
    container(container(content).padding(10).style(container::rounded_box))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(40)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgba(
                    0.0, 0.0, 0.0, 0.55,
                ))),
                text_color: Some(palette.background.base.text),
                ..container::Style::default()
            }
        })
        .into()
}
