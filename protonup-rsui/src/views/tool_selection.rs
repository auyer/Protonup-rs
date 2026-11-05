use iced::Element;
use iced::widget::{Column, scrollable, text};

use crate::message::{Focusable, GuiMode, Message, Pane};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

use super::focus_style::{self, Highlight};

fn title(state: &ProtonupGui) -> String {
    match state.mode {
        GuiMode::CheckWhatsNew => "Select a tool to check its changelog:".to_string(),
        _ => {
            let app_name = match state.mode {
                GuiMode::DownloadForSteam => "Steam",
                GuiMode::DownloadForLutris => "Lutris",
                _ => "App",
            };
            format!("Select tool for {}:", app_name)
        }
    }
}

/// Focusable options: one tool per row.
pub(crate) fn main_focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    state
        .available_tools
        .iter()
        .enumerate()
        .map(|(index, _)| vec![Focusable::new(Message::ToolChosen(index))])
        .collect()
}

pub(crate) fn actions(_state: &ProtonupGui) -> Vec<Action> {
    vec![Action::new("Back", Message::BackToInitial)]
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);
    column = column.push(text(title(state)).size(16));

    if state.available_tools.is_empty() {
        column = column.push(text("Loading tools...").size(14));
    } else {
        for (index, tool) in state.available_tools.iter().enumerate() {
            let highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, index, 0));

            column = column.push(focus_style::option_list_row(
                text(&tool.name).size(14),
                highlight,
                Some(Message::ToolChosen(index)),
                action_bar::LIST_BUTTON_WIDTH,
            ));
        }
    }

    let content: Element<'_, Message> = scrollable(column).id(super::scroll_id::main()).into();

    action_bar::with_actions(content, actions(state), |col| {
        focus_style::action_focused(state, col)
    })
}
