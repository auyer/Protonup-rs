use iced::widget::{Column, Container, Row, button, container, text};
use iced::{Center, Element, Length};

use crate::message::Message;
use crate::state::warning_button_style;

/// Visual style of an action button in the bottom action pane.
pub(crate) enum ActionStyle {
    Normal,
    Warning,
}

/// A single button in the bottom action pane.
///
/// `message == None` renders a disabled (non-pressable) button.
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

/// Wraps the view content in a layout where the content occupies the remaining
/// vertical space (and scrolls if it is the scrollable body of a view) while
/// the given actions stay pinned in a horizontal pane at the bottom, always
/// visible regardless of content length.
pub(crate) fn with_actions<'a>(
    content: impl Into<Element<'a, Message>>,
    actions: impl IntoIterator<Item = Action>,
) -> Element<'a, Message> {
    let mut row = Row::new().spacing(10).align_y(Center);

    for action in actions {
        let button = button(text(action.label).size(14)).padding(10);

        let button = match action.style {
            ActionStyle::Warning => button.style(warning_button_style()),
            ActionStyle::Normal => button,
        };

        let element: Element<'_, Message> = match action.message {
            Some(message) => button.on_press(message).into(),
            None => button.into(),
        };

        row = row.push(element);
    }

    let action_pane = Container::new(row)
        .width(Length::Fill)
        .padding([10, 15])
        .style(container::rounded_box);

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
