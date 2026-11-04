use iced::widget::{Column, Container, Row, button, text};
use iced::{Center, Element, Length};

use crate::message::Message;
use crate::state::warning_button_style;

use super::focus_style::{self, Highlight};

/// Visual style of an action button in the bottom action pane.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ActionStyle {
    Normal,
    Warning,
}

/// A single button in the bottom action pane.
///
/// `message == None` renders a disabled (non-pressable) button.
#[derive(Debug, Clone)]
pub(crate) struct Action {
    pub label: String,
    pub message: Option<Message>,
    pub style: ActionStyle,
}

impl Action {
    /// Enabled button with a label and the message sent on press.
    pub fn new(label: impl Into<String>, message: Message) -> Self {
        Self {
            label: label.into(),
            message: Some(message),
            style: ActionStyle::Normal,
        }
    }

    /// Disabled button (no message on press).
    pub fn disabled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            message: None,
            style: ActionStyle::Normal,
        }
    }

    /// Mark the button with the warning style.
    pub fn warning(mut self) -> Self {
        self.style = ActionStyle::Warning;
        self
    }
}

/// Width (in logical pixels) of list-selection buttons.
///
/// iced 0.14 has no "max" length, so all list buttons share this uniform width.
pub(crate) const LIST_BUTTON_WIDTH: f32 = 220.0;

/// Wraps the view content in a layout where the content occupies the remaining
/// vertical space (and scrolls if it is the scrollable body of a view) while
/// the given actions stay pinned in a horizontal pane at the bottom, always
/// visible regardless of content length.
///
/// Enabled actions are numbered left-to-right; the `col` passed to
/// `is_focused` matches their index in [`crate::input::pane_grid`].
pub(crate) fn with_actions<'a>(
    content: impl Into<Element<'a, Message>>,
    actions: impl IntoIterator<Item = Action>,
    is_focused: impl Fn(usize) -> bool,
) -> Element<'a, Message> {
    let mut row = Row::new().spacing(10).align_y(Center);
    let mut col = 0usize;

    for action in actions {
        let focused = action.message.is_some() && is_focused(col);
        if action.message.is_some() {
            col += 1;
        }

        let button = button(text(action.label).size(14)).padding(10);

        let button = match action.style {
            ActionStyle::Warning => button.style(warning_button_style()),
            ActionStyle::Normal => button,
        };

        let element: Element<'_, Message> = match action.message {
            Some(message) => button.on_press(message).into(),
            None => button.into(),
        };

        row = row.push(focus_style::focus_frame(element, Highlight::from(focused)));
    }

    let action_pane = Container::new(row)
        .width(Length::Fill)
        .padding([10, 15])
        .style(focus_style::action_bar_style(Highlight::from(is_focused(
            0,
        ))));

    let content: Element<'a, Message> = content.into();
    let content_pane = Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill);

    Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(content_pane)
        .push(action_pane)
        .into()
}

/// Like [`with_actions`] for views that render no action bar at all.
pub(crate) fn without_actions<'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
