use iced::widget::{Column, Row, button, radio, scrollable, text};
use iced::{Center, Element};

use crate::message::{GuiMode, Message};
use crate::state::ProtonupGui;

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
            column = column.push(
                Row::new()
                    .spacing(10)
                    .align_y(Center)
                    .push(radio(
                        "",
                        index,
                        state.selected_tool_indices.first().copied(),
                        Message::ToolSelected,
                    ))
                    .push(text(&tool.name).size(14)),
            );
        }
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
