use iced::Element;
use iced::widget::{Column, button, scrollable, text, text_input};

use crate::message::Message;
use crate::state::ProtonupGui;

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

    column = column.push(
        button(text("Continue").size(14))
            .on_press(Message::ToolSelectionConfirmed)
            .padding(10),
    );

    column = column.push(
        button(text("Back").size(14))
            .on_press(Message::BackToInitial)
            .padding(10),
    );

    scrollable(column).into()
}
