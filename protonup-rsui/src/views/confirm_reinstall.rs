use iced::Element;
use iced::widget::{Column, Row, checkbox, scrollable, text};

use crate::message::Message;
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

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

    let content: Element<'_, Message> = scrollable(column).into();

    action_bar::with_actions(
        content,
        [
            Action::new("Continue", Message::ConfirmReinstallSelection),
            Action::new("Back", Message::BackToInitial),
        ],
    )
}
