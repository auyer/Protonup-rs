use iced::Element;
use iced::widget::{Column, Row, button, checkbox, scrollable, text};

use crate::message::Message;
use crate::state::ProtonupGui;

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let tool_name = state
        .selected_tool
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("Tool");

    let mut column = Column::new().spacing(10);
    column = column.push(text(format!("Select versions for {}:", tool_name)).size(16));

    if state.available_versions.is_empty() {
        column = column.push(text("Loading versions...").size(14));
    } else {
        for (index, release) in state.available_versions.iter().enumerate() {
            let is_selected = state.selected_version_indices.contains(&index);
            column = column.push(
                Row::new()
                    .spacing(10)
                    .push(checkbox(is_selected).on_toggle(move |_| Message::ToggleVersion(index)))
                    .push(text(&release.tag_name).size(14)),
            );
        }
    }

    column = column.push(
        button(text("Start Download").size(14))
            .on_press(Message::StartSelectedDownloads)
            .padding(10),
    );

    column = column.push(
        button(text("Back").size(14))
            .on_press(Message::BackToToolSelection)
            .padding(10),
    );

    scrollable(column).into()
}
