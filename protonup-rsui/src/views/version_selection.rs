use iced::widget::{Column, Container, Row, scrollable, text};
use iced::{Element, Length};

use crate::message::{Focusable, GuiMode, Message, Pane};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};
use crate::views::changelog;

use super::focus_style::{self, Highlight};

/// The message that opens the changelog for the currently selected version.
fn changelog_message(state: &ProtonupGui) -> Message {
    let release = state
        .selected_version_indices
        .last()
        .and_then(|&i| state.available_versions.get(i))
        .or_else(|| state.available_versions.first())
        .cloned();
    let tool = state.selected_tool.clone();
    Message::ToggleChangelog(Box::new(release.zip(tool)))
}

pub(crate) fn actions(state: &ProtonupGui) -> Vec<Action> {
    if state.mode == GuiMode::CheckWhatsNew {
        vec![Action::new("Back", Message::BackToToolSelection)]
    } else {
        vec![
            Action::new("Start Download", Message::StartSelectedDownloads),
            Action::new("Show Changelog", changelog_message(state)),
            Action::new("Back", Message::BackToToolSelection),
        ]
    }
}

/// Focusable options: one version per row, plus a `Close` row for the open
/// changelog pane (which is part of the main pane).
pub(crate) fn main_focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    let mut rows: Vec<Vec<Focusable>> = state
        .available_versions
        .iter()
        .enumerate()
        .map(|(index, _)| {
            vec![Focusable::new(if state.mode == GuiMode::CheckWhatsNew {
                Message::VersionChosen(index)
            } else {
                Message::ToggleVersion(index)
            })]
        })
        .collect();

    if state.show_changelog.is_some() {
        rows.push(vec![Focusable::new(Message::ToggleChangelog(None.into()))]);
    }

    rows
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let tool_name = state
        .selected_tool
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("Tool");

    let mut left = Column::new().spacing(10).width(Length::Fill);
    left = left.push(text(format!("Select versions for {}:", tool_name)).size(16));

    let changelog_open = state.show_changelog.is_some();

    if state.available_versions.is_empty() {
        left = left.push(text("Loading versions...").size(14));
    } else if state.mode == GuiMode::CheckWhatsNew {
        for (index, release) in state.available_versions.iter().enumerate() {
            let highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, index, 0));

            left = left.push(focus_style::option_list_row(
                text(&release.tag_name).size(14),
                highlight,
                Some(Message::VersionChosen(index)),
                action_bar::LIST_BUTTON_WIDTH,
            ));
        }
    } else {
        for (index, release) in state.available_versions.iter().enumerate() {
            let is_selected = state.selected_version_indices.contains(&index);
            let highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, index, 0));

            left = left.push(focus_style::option_checkbox_row(
                is_selected,
                &release.tag_name,
                highlight,
                Message::ToggleVersion(index),
                action_bar::LIST_BUTTON_WIDTH,
            ));
        }
    }

    if changelog_open {
        let close_highlight = Highlight::from(focus_style::is_focused(
            state,
            Pane::Main,
            state.available_versions.len(),
            0,
        ));
        left = left.push(focus_style::focus_frame(
            iced::widget::button(text("Hide release notes").size(12))
                .on_press(Message::ToggleChangelog(None.into()))
                .padding(5),
            close_highlight,
        ));
    }

    let left_element: Element<_> = scrollable(left).id(super::scroll_id::main()).into();

    let content: Element<'_, Message> =
        if let Some((ref release, ref compat_tool)) = state.show_changelog {
            let right = Container::new(changelog::view(release, compat_tool))
                .width(Length::FillPortion(2))
                .padding(10);

            let left_portion = Container::new(left_element).width(Length::FillPortion(1));

            Row::new().push(left_portion).push(right).into()
        } else {
            left_element
        };

    action_bar::with_actions(content, actions(state), |col| {
        focus_style::action_focused(state, col)
    })
}
