use iced::Element;
use iced::widget::{Column, Row, button, checkbox, scrollable, text};

use crate::message::Message;
use crate::state::ProtonupGui;

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("The following tools are already installed:").size(16));

    column = column.push(text("Select which ones you want to reinstall:").size(14));

    if state.already_installed_tools.is_empty() {
        column = column.push(text("No tools to reinstall.").size(14));
    } else {
        for (index, tool) in state.already_installed_tools.iter().enumerate() {
            let is_selected = state.force_reinstall_indices.contains(&index);
            column = column.push(
                Row::new()
                    .spacing(10)
                    .push(checkbox(is_selected).on_toggle(move |_| Message::ToggleReinstall(index)))
                    .push(text(&tool.name).size(14)),
            );
        }
    }

    column = column.push(
        button(text("Continue").size(14))
            .on_press(Message::ConfirmReinstallSelection)
            .padding(10),
    );

    column = column.push(
        button(text("Back").size(14))
            .on_press(Message::BackToInitial)
            .padding(10),
    );

    scrollable(column).into()
}
