use iced::widget::{button, checkbox, radio, scrollable, text, Column, Row, Container};
use iced::{Element, Length};

use crate::message::{GuiMode, Message};
use crate::state::ProtonupGui;
use crate::views::changelog;

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let tool_name = state
        .selected_tool
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("Tool");

    let mut left = Column::new().spacing(10).width(Length::Fill);
    left = left.push(text(format!("Select versions for {}:", tool_name)).size(16));

    if state.available_versions.is_empty() {
        left = left.push(text("Loading versions...").size(14));
    } else if state.mode == GuiMode::CheckWhatsNew {
        for (index, release) in state.available_versions.iter().enumerate() {
            left = left.push(
                Row::new()
                    .spacing(10)
                    .push(radio(
                        "",
                        index,
                        state.selected_version_indices.first().copied(),
                        Message::ToggleVersion,
                    ))
                    .push(text(&release.tag_name).size(14)),
            );
        }
    } else {
        for (index, release) in state.available_versions.iter().enumerate() {
            let is_selected = state.selected_version_indices.contains(&index);
            left = left.push(
                Row::new()
                    .spacing(10)
                    .push(checkbox(is_selected).on_toggle(move |_| Message::ToggleVersion(index)))
                    .push(text(&release.tag_name).size(14)),
            );
        }
    }

    if state.mode == GuiMode::CheckWhatsNew {
        left = left.push({
            let release = state
                .selected_version_indices
                .last()
                .and_then(|&i| state.available_versions.get(i))
                .or_else(|| state.available_versions.first())
                .cloned();
            let tool = state.selected_tool.clone();
            button(text("View Changelog").size(14))
                .on_press_with(move || {
                    Message::ToggleChangelog(
                        release.clone().and_then(|r| tool.clone().map(|t| (r, t))),
                    )
                })
                .padding(10)
        });

        left = left.push(
            button(text("Back").size(14))
                .on_press(Message::BackToToolSelection)
                .padding(10),
        );
    } else {

        left = left.push(
            button(text("Start Download").size(14))
                .on_press(Message::StartSelectedDownloads)
                .padding(10),
        );

        left = left.push({
            let release = state
                .selected_version_indices
                .last()
                .and_then(|&i| state.available_versions.get(i))
                .or_else(|| state.available_versions.first())
                .cloned();
            let tool = state.selected_tool.clone();
            button(text("Show Changelog").size(14))
                .on_press_with(move || {
                    Message::ToggleChangelog(
                        release.clone().and_then(|r| tool.clone().map(|t| (r, t))),
                    )
                })
                .padding(10)
        });

        left = left.push(
            button(text("Back").size(14))
                .on_press(Message::BackToToolSelection)
                .padding(10),
        );
    }

    let left_element: Element<_> = scrollable(left).into();

    if let Some((ref release, ref compat_tool)) = state.show_changelog {
        let right = Container::new(changelog::view(release, compat_tool))
            .width(Length::FillPortion(2))
            .padding(10);

        let left_portion = Container::new(left_element).width(Length::FillPortion(1));

        Row::new().push(left_portion).push(right).into()
    } else {
        left_element
    }
}
