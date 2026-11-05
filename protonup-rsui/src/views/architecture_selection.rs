use iced::Element;
use iced::widget::{Column, scrollable, text};

use libprotonup::architecture_variants::MicroArchVariants;

use crate::message::{Focusable, Message, Pane};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

use super::focus_style::{self, Highlight};

/// The architecture variants offered, in rendering order.
pub(crate) const VARIANTS: [MicroArchVariants; 4] = [
    MicroArchVariants::X86_64,
    MicroArchVariants::X86_64V2,
    MicroArchVariants::X86_64V3,
    MicroArchVariants::X86_64V4,
];

pub(crate) fn main_focusable_options(_state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    VARIANTS
        .iter()
        .map(|&variant| vec![Focusable::new(Message::ArchitectureChosen(variant))])
        .collect()
}

pub(crate) fn actions(_state: &ProtonupGui) -> Vec<Action> {
    vec![Action::new("Back", Message::VersionsFetched(vec![]))]
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("Select CPU Architecture Variant:").size(16));

    column = column
        .push(text("Some tools offer optimized builds for different CPU architectures.").size(12));

    for (index, variant) in VARIANTS.iter().enumerate() {
        let highlight = Highlight::from(focus_style::is_focused(state, Pane::Main, index, 0));

        column = column.push(focus_style::option_list_row(
            Column::new()
                .push(text(variant.name()).size(14))
                .push(text(variant.description()).size(10)),
            highlight,
            Some(Message::ArchitectureChosen(*variant)),
            action_bar::LIST_BUTTON_WIDTH,
        ));
    }

    let content: Element<'_, Message> = scrollable(column).id(super::scroll_id::main()).into();

    action_bar::with_actions(content, actions(state), |col| {
        focus_style::action_focused(state, col)
    })
}
