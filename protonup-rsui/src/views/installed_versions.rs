use iced::Element;
use iced::widget::{Column, Row, button, container, rule, scrollable, text};

use crate::message::{InstalledTool, Message};

/// Side pane listing all installed versions for one app (changelog-style).
pub(crate) fn view(tool: &InstalledTool) -> Element<'_, Message> {
    let mut column = Column::new().spacing(8);

    column = column.push(text(format!("{}", tool.app)).size(14));
    column = column.push(text("Installed versions:").size(12));
    column = column.push(rule::horizontal(1));

    if tool.versions.is_empty() {
        column = column.push(text("No versions found").size(12));
    } else {
        for version in &tool.versions {
            column = column.push(
                Row::new()
                    .spacing(10)
                    .push(text("•").size(12))
                    .push(text(version.as_str()).size(12)),
            );
        }
    }

    column = column.push(
        button(text("Close").size(12))
            .on_press(Message::ToggleInstalledVersions(Box::new(None)))
            .padding(5),
    );

    container(scrollable(column).id(crate::views::scroll_id::main_right()))
        .padding(10)
        .style(container::rounded_box)
        .into()
}
