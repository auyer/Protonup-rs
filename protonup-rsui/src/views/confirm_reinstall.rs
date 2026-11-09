use iced::Element;
use iced::widget::{Column, scrollable, text};

use crate::message::{Focusable, Message, Pane};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

use super::focus_style::{self, Highlight};

pub(crate) fn main_focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    state
        .already_installed_tools
        .iter()
        .enumerate()
        .map(|(index, _)| vec![Focusable::new(Message::ToggleReinstall(index))])
        .collect()
}

pub(crate) fn actions(_state: &ProtonupGui) -> Vec<Action> {
    vec![
        Action::new("Continue", Message::ConfirmReinstallSelection),
        Action::new("Back", Message::BackToInitial),
    ]
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("The following tools are already installed:").size(16));

    column = column.push(text("Select which ones you want to reinstall:").size(14));

    if state.already_installed_tools.is_empty() {
        column = column.push(text("No tools to reinstall.").size(14));
    } else {
        for (index, tool) in state.already_installed_tools.iter().enumerate() {
            let is_selected = state.force_reinstall_indices.contains(&index);
            let highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, index, 0));

            column = column.push(focus_style::option_checkbox_row(
                is_selected,
                &tool.name,
                highlight,
                Message::ToggleReinstall(index),
                action_bar::LIST_BUTTON_WIDTH,
            ));
        }
    }

    let content: Element<'_, Message> = scrollable(column).id(super::scroll_id::main()).into();

    action_bar::with_actions(content, actions(state), |col| {
        focus_style::action_focused(state, col)
    })
}
