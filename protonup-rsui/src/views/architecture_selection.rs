use iced::widget::{Column, Row, button, checkbox, scrollable, text};
use iced::{Center, Element};

use libprotonup::architecture_variants::MicroArchVariants;

use crate::message::Message;
use crate::state::ProtonupGui;

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

    column = column.push(
        button(text("Continue").size(14))
            .on_press(Message::StartSelectedDownloads)
            .padding(10),
    );

    column = column.push(
        button(text("Back").size(14))
            .on_press(Message::VersionsFetched(vec![]))
            .padding(10),
    );

    scrollable(column).into()
}
