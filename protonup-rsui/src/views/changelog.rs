use iced::widget::{button, container, rule, scrollable, text, Column};
use iced::Element;

use libprotonup::downloads::Release;
use libprotonup::sources::CompatTool;

use crate::message::Message;

pub(crate) fn view<'a>(release: &'a Release, compat_tool: &'a CompatTool) -> Element<'a, Message> {
    let url = format!(
        "{}{}/{}/releases/tag/{}",
        compat_tool.forge.get_user_url(),
        compat_tool.repository_account,
        compat_tool.repository_name,
        release.tag_name
    );

    let mut column = Column::new().spacing(8);

    column = column.push(
        text(format!("{} {}", compat_tool.name, release.tag_name))
            .size(16),
    );

    column = column.push(text(url).size(10));

    column = column.push(rule::horizontal(1));

    match &release.body {
        Some(body) => {
            column = column.push(text(body.as_str()).size(12));
        }
        None => {
            column = column.push(text("(no release notes)").size(12));
        }
    }

    column = column.push(
        button(text("Close").size(12))
            .on_press(Message::ToggleChangelog(None))
            .padding(5),
    );

    container(scrollable(column))
        .padding(10)
        .style(container::rounded_box)
        .into()
}
