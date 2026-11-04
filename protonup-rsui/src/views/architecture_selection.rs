use iced::widget::{Column, Row, checkbox, scrollable, text};
use iced::{Center, Element};

use libprotonup::architecture_variants::MicroArchVariants;

use crate::message::Message;
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("Select CPU Architecture Variant:").size(16));

    column = column
        .push(text("Some tools offer optimized builds for different CPU architectures.").size(12));

    let variants = [
        MicroArchVariants::X86_64,
        MicroArchVariants::X86_64V2,
        MicroArchVariants::X86_64V3,
        MicroArchVariants::X86_64V4,
    ];

    for variant in variants {
        let is_selected = state.selected_arch_variant == Some(variant);
        column = column.push(
            Row::new()
                .spacing(10)
                .align_y(Center)
                .push(
                    checkbox(is_selected).on_toggle(move |_| Message::SelectArchitecture(variant)),
                )
                .push(
                    Column::new()
                        .push(text(variant.name()).size(14))
                        .push(text(variant.description()).size(10)),
                ),
        );
    }

    let content: Element<'_, Message> = scrollable(column).into();

    action_bar::with_actions(
        content,
        [
            Action::new("Continue", Message::StartSelectedDownloads),
            Action::new("Back", Message::VersionsFetched(vec![])),
        ],
    )
}
