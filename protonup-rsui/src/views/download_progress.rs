use iced::Element;
use iced::widget::{Column, button, progress_bar, text};

use crate::message::{Message, ToolStatus};
use crate::state::ProtonupGui;

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    for tool in &state.tools {
        let status_color = match &tool.status {
            ToolStatus::_Complete => [0.3, 1.0, 0.3],
            ToolStatus::Error(_) => [1.0, 0.3, 0.3],
            _ => [1.0, 1.0, 1.0],
        };

        column = column.push(
            Column::new()
                .spacing(5)
                .push(text(tool.name.to_string()).size(12))
                .push(progress_bar(0.0..=100.0, tool.progress))
                .push(text(tool.status_text()).size(10).color(status_color)),
        );
    }

    if let Some(ref result) = state.download_complete {
        match result {
            Ok(versions) => {
                column =
                    column.push(text("✓ All tools installed successfully!").color([0.3, 1.0, 0.3]));
                for version in versions {
                    column = column.push(text(format!("  • {}", version)).size(12));
                }
                column = column.push(
                    button(text("Back to Main Menu").size(14))
                        .on_press(Message::BackToInitial)
                        .padding(5),
                );
            }
            Err(e) => {
                column = column.push(text(format!("✗ Failed: {}", e)).color([1.0, 0.3, 0.3]));
                column = column.push(
                    button(text("Try Again").size(14))
                        .on_press(Message::StartSelectedDownloads)
                        .padding(5),
                );
            }
        }
    }

    column.into()
}
