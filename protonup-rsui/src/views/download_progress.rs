use iced::Element;
use iced::widget::{Column, Row, button, progress_bar, scrollable, text};

use libprotonup::downloads::Release;
use libprotonup::sources::CompatTool;

use crate::message::{Focusable, Message, Pane, ToolStatus};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

use super::focus_style::{self, Highlight};

/// The changelog buttons rendered for each completed tool, in rendering order.
fn changelog_buttons(state: &ProtonupGui) -> Vec<Message> {
    state
        .tools
        .iter()
        .filter(|tool| tool.status == ToolStatus::_Complete)
        .filter_map(|tool| {
            find_release_pair(state, &tool.name).map(|(release, compat_tool)| {
                Message::ToggleChangelog(Box::new(Some((release.clone(), compat_tool.clone()))))
            })
        })
        .collect()
}

/// Focusable options: one changelog button per completed tool.
pub(crate) fn main_focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    changelog_buttons(state)
        .into_iter()
        .map(|message| vec![Focusable::new(message)])
        .collect()
}

pub(crate) fn actions(state: &ProtonupGui) -> Vec<Action> {
    match &state.download_complete {
        Some(Ok(_)) => vec![Action::new("Back to Main Menu", Message::BackToInitial)],
        Some(Err(_)) => vec![Action::new("Try Again", Message::StartSelectedDownloads)],
        None => Vec::new(),
    }
}

/// The primary "next" action of the download view.
pub(crate) fn primary_action(state: &ProtonupGui) -> Option<Message> {
    match &state.download_complete {
        Some(Ok(_)) => Some(Message::BackToInitial),
        Some(Err(_)) => Some(Message::StartSelectedDownloads),
        None => None,
    }
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    let changelog_messages = changelog_buttons(state);
    let mut changelog_index = 0usize;

    for tool in &state.tools {
        let status_color = match &tool.status {
            ToolStatus::_Complete => [0.3, 1.0, 0.3],
            ToolStatus::Error(_) => [1.0, 0.3, 0.3],
            _ => [1.0, 1.0, 1.0],
        };

        let mut row = Row::new().spacing(10).push(
            Column::new()
                .spacing(5)
                .push(text(tool.name.to_string()).size(12))
                .push(progress_bar(0.0..=100.0, tool.progress))
                .push(text(tool.status_text()).size(10).color(status_color)),
        );

        if tool.status == ToolStatus::_Complete
            && let Some(message) = changelog_messages.get(changelog_index)
        {
            let highlight = Highlight::from(focus_style::is_focused(
                state,
                Pane::Main,
                changelog_index,
                0,
            ));
            changelog_index += 1;

            row = row.push(focus_style::focus_frame(
                button(text("Changelog").size(10))
                    .on_press(message.clone())
                    .padding(3),
                highlight,
            ));
        }

        column = column.push(row);
    }

    if let Some(ref result) = state.download_complete {
        match result {
            Ok(pairs) => {
                column =
                    column.push(text("✓ All tools installed successfully!").color([0.3, 1.0, 0.3]));
                for (release, _) in pairs {
                    column = column.push(text(format!("  • {}", release.tag_name)).size(12));
                }
            }
            Err(e) => {
                column = column.push(text(format!("✗ Failed: {}", e)).color([1.0, 0.3, 0.3]));
            }
        }
    }

    let content: Element<'_, Message> = scrollable(column).into();

    let actions = actions(state);
    if actions.is_empty() {
        action_bar::without_actions(content)
    } else {
        action_bar::with_actions(content, actions, |col| {
            focus_style::action_focused(state, col)
        })
    }
}

fn find_release_pair<'a>(
    state: &'a ProtonupGui,
    tool_name: &str,
) -> Option<&'a (Release, CompatTool)> {
    let result = state.download_complete.as_ref()?.as_ref().ok()?;
    result.iter().find(|(release, compat_tool)| {
        tool_name.starts_with(compat_tool.name.as_str())
            && tool_name.contains(release.tag_name.as_str())
    })
}
