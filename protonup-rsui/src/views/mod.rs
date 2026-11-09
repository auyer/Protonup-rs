use iced::widget::{Column, Row, container, text};
use iced::{Center, Element, Length};

use crate::message::Message;
use crate::state::ProtonupGui;

/// Height reserved by the header bar, in logical pixels.
pub(crate) const HEADER_HEIGHT: f32 = 44.0;

pub mod action_bar;
pub mod changelog;
pub mod confirm_reinstall;
pub mod custom_location;
pub mod download_progress;
pub mod focus_style;
pub mod footer;
pub mod installed_versions;
pub mod manage_installations;
pub mod quick_update;
pub mod scroll_id;
pub mod selection_flow;
pub mod shortcuts;
pub mod sidebar;
pub mod tool_selection;
pub mod tool_variant_selection;
pub mod version_selection;

/// The actions of the bottom action pane for the current state.
///
/// Single source of truth shared by the renderer and by
/// [`crate::input::pane_grid`], so keyboard/controller focus order always
/// matches what is on screen.
pub(crate) fn main_content_actions(state: &ProtonupGui) -> Vec<action_bar::Action> {
    selection_flow::view_actions(state)
}

pub(crate) fn app_view(state: &ProtonupGui) -> Element<'_, Message> {
    // A shortcut panel temporarily replaces the whole window content.
    if state.show_keyboard_shortcuts {
        return shortcuts::keyboard_view();
    }
    if state.show_controller_shortcuts {
        return shortcuts::controller_view();
    }

    // The header is fixed-height so it always reserves space, even when the
    // viewport is small (e.g. gamescope on a Steam Deck).
    let header = container(
        Row::new()
            .push(text("Protonup-rs").size(20))
            .push(iced::widget::space::horizontal())
            .push(text(&state.global_status).size(12))
            .padding(10)
            .align_y(Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(HEADER_HEIGHT))
    .style(focus_style::panel_style());

    let sidebar = self::sidebar::sidebar(state);
    let main_content = selection_flow::view_main_content(state);

    // Only the main content stretches; the footer keeps its reserved height so
    // it can never be pushed off-screen by a tall content pane.
    let main_column = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(
            container(main_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(20),
        )
        .push(footer::view());

    Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(header)
        .push(
            Row::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(sidebar)
                .push(main_column),
        )
        .into()
}
