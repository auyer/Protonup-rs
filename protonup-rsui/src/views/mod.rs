use iced::widget::{Column, Row, container, text};
use iced::{Center, Element};

use crate::message::Message;
use crate::state::ProtonupGui;

pub mod architecture_selection;
pub mod changelog;
pub mod confirm_reinstall;
pub mod custom_location;
pub mod download_progress;
pub mod manage_installations;
pub mod quick_update;
pub mod selection_flow;
pub mod sidebar;
pub mod tool_selection;
pub mod version_selection;

pub(crate) fn app_view(state: &ProtonupGui) -> Element<'_, Message> {
    let header = container(
        Row::new()
            .push(text("Protonup-rs").size(20))
            .push(iced::widget::space::horizontal())
            .push(text(&state.global_status).size(12))
            .padding(10)
            .align_y(Center),
    )
    .style(|theme: &iced::Theme| {
        let palette = theme.extended_palette();
        container::Style::default()
            .border(iced::border::color(palette.background.strong.color).width(1))
    });

    let sidebar = self::sidebar::sidebar(state);
    let main_content = selection_flow::view_main_content(state);

    Column::new()
        .push(header)
        .push(Row::new().push(sidebar).push(main_content))
        .into()
}
