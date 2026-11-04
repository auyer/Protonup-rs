use iced::Element;
use iced::widget::{Column, button, scrollable, text, text_input};

use crate::message::Message;
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("Select Installation Directory:").size(16));

    column = column.push(
        text("Enter a path or use the folder picker to select where compatibility tools will be installed.").size(12)
    );

    column = column.push(
        text_input("Enter path...", &state.custom_path_input)
            .on_input(Message::CustomPathInput)
            .padding(10),
    );

    column = column.push(
        button(text("📁 Browse...").size(14))
            .on_press(Message::OpenFolderPicker)
            .padding(10),
    );

    if let Some(ref error) = state.path_error {
        column = column.push(text(error).size(12).color([1.0, 0.3, 0.3]));
    }

    let content: Element<'_, Message> = scrollable(column).into();

    action_bar::with_actions(
        content,
        [
            Action::new("Continue", Message::ToolSelectionConfirmed),
            Action::new("Back", Message::BackToInitial),
        ],
    )
}
