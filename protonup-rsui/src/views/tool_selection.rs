use iced::Element;
use iced::widget::{Column, scrollable, text};

use crate::message::{GuiMode, Message};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let title = match state.mode {
        GuiMode::CheckWhatsNew => "Select a tool to check its changelog:".to_string(),
        _ => {
            let app_name = match state.mode {
                GuiMode::DownloadForSteam => "Steam",
                GuiMode::DownloadForLutris => "Lutris",
                _ => "App",
            };
            format!("Select tool for {}:", app_name)
        }
    };

    let mut column = Column::new().spacing(10);
    column = column.push(text(title).size(16));

    if state.available_tools.is_empty() {
        column = column.push(text("Loading tools...").size(14));
    } else {
        for (index, tool) in state.available_tools.iter().enumerate() {
            column = column.push(action_bar::list_button(
                text(&tool.name).size(14),
                Message::ToolChosen(index),
            ));
        }
    }

    let content: Element<'_, Message> = scrollable(column).into();

    action_bar::with_actions(content, [Action::new("Back", Message::BackToInitial)])
}
