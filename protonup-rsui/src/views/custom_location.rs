use iced::Element;
use iced::widget::{Column, button, scrollable, text, text_input};

use crate::message::{Focusable, Message, Pane};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

use super::focus_style::{self, Highlight};

/// iced widget id of the custom path input, used for programmatic focus.
pub(crate) fn path_input_id() -> iced::widget::Id {
    iced::widget::Id::new("custom-path-input")
}

pub(crate) fn main_focusable_options(_state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    vec![
        // The text input is focused as a widget, not activated as a message.
        vec![Focusable::widget(path_input_id())],
        vec![Focusable::new(Message::OpenFolderPicker)],
    ]
}

pub(crate) fn actions(_state: &ProtonupGui) -> Vec<Action> {
    vec![
        Action::new("Continue", Message::ToolSelectionConfirmed),
        Action::new("Back", Message::BackToInitial),
    ]
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("Select Installation Directory:").size(16));

    column = column.push(
        text("Enter a path or use the folder picker to select where compatibility tools will be installed.").size(12)
    );

    let path_highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, 0, 0));
    column = column.push(focus_style::focus_frame(
        text_input("Enter path...", &state.custom_path_input)
            .id(path_input_id())
            .on_input(Message::CustomPathInput)
            .style(focus_style::text_input_style(path_highlight))
            .padding(10),
        Highlight::None,
    ));

    let browse_highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, 1, 0));
    column = column.push(focus_style::focus_frame(
        button(text("📁 Browse...").size(14))
            .on_press(Message::OpenFolderPicker)
            .padding(10),
        browse_highlight,
    ));

    if let Some(ref error) = state.path_error {
        column = column.push(text(error).size(12).color([1.0, 0.3, 0.3]));
    }

    let content: Element<'_, Message> = scrollable(column).id(super::scroll_id::main()).into();

    action_bar::with_actions(content, actions(state), |col| {
        focus_style::action_focused(state, col)
    })
}
