use iced::Element;
use iced::widget::{Column, scrollable, text};

use crate::message::{Focusable, Message, Pane};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

use super::focus_style::{self, Highlight};

/// Width of a variant row.
///
/// Wider than the regular list buttons because each row also shows the archive
/// file name, which is long and must not be clipped.
const VARIANT_BUTTON_WIDTH: f32 = 420.0;

pub(crate) fn main_focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    state
        .available_variants
        .iter()
        .enumerate()
        .map(|(index, _)| vec![Focusable::new(Message::VariantChosen(index))])
        .collect()
}

pub(crate) fn actions(_state: &ProtonupGui) -> Vec<Action> {
    vec![Action::new("Back", Message::VersionsFetched(vec![]))]
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("Select a variant:").size(16));

    column = column.push(
        text("Some tools ship several builds of the same release. Pick the one to install.")
            .size(12),
    );

    if state.available_variants.is_empty() {
        column = column.push(text("No variants available.").size(14));
    }

    for (index, variant) in state.available_variants.iter().enumerate() {
        let highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, index, 0));

        column = column.push(focus_style::option_list_row(
            Column::new()
                .push(text(variant.label()).size(14))
                .push(text(variant.file_name()).size(10)),
            highlight,
            Some(Message::VariantChosen(index)),
            VARIANT_BUTTON_WIDTH,
        ));
    }

    let content: Element<'_, Message> = scrollable(column).id(super::scroll_id::main()).into();

    action_bar::with_actions(content, actions(state), |col| {
        focus_style::action_focused(state, col)
    })
}
