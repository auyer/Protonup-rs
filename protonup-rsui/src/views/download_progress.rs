use iced::Element;
use iced::widget::{Column, Row, button, progress_bar, scrollable, text};

use libprotonup::downloads::Release;
use libprotonup::sources::CompatTool;

use crate::message::{Message, ToolStatus};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

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
            && let Some((release, compat_tool)) = find_release_pair(state, &tool.name)
        {
            row = row.push(
                button(text("Changelog").size(10))
                    .on_press(Message::ToggleChangelog(Box::new(Some((
                        release.clone(),
                        compat_tool.clone(),
                    )))))
                    .padding(3),
            );
        }

        column = column.push(row);
    }

    let mut actions: Vec<Action> = Vec::new();

    if let Some(ref result) = state.download_complete {
        match result {
            Ok(pairs) => {
                column =
                    column.push(text("✓ All tools installed successfully!").color([0.3, 1.0, 0.3]));
                for (release, _) in pairs {
                    column = column.push(text(format!("  • {}", release.tag_name)).size(12));
                }
                actions.push(Action::new("Back to Main Menu", Message::BackToInitial));
            }
            Err(e) => {
                column = column.push(text(format!("✗ Failed: {}", e)).color([1.0, 0.3, 0.3]));
                actions.push(Action::new("Try Again", Message::StartSelectedDownloads));
            }
        }
    }

    let content: Element<'_, Message> = scrollable(column).into();

    if actions.is_empty() {
        content
    } else {
        action_bar::with_actions(content, actions)
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
