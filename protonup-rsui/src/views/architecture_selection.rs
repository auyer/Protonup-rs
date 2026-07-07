use iced::widget::{Column, Row, button, checkbox, scrollable, text};
use iced::{Center, Element};

use crate::message::Message;
use crate::state::ProtonupGui;

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10);

    column = column.push(text("Select CPU Architecture Variant:").size(16));

    column = column
        .push(text("Some tools offer optimized builds for different CPU architectures.").size(12));

    let variants = [
        (1, "x86_64", "Universal - all x86-64 CPUs"),
        (2, "x86_64_v2", "Recommended - optimized for SSE3"),
        (3, "x86_64_v3", "Modern CPUs - optimized for AVX2"),
        (4, "x86_64_v4", "Experimental - optimized for AVX-512"),
    ];

    for (code, name, desc) in variants {
        let is_selected = state.selected_arch_variant == Some(code);
        column = column.push(
            Row::new()
                .spacing(10)
                .align_y(Center)
                .push(checkbox(is_selected).on_toggle(move |_| Message::SelectArchitecture(code)))
                .push(
                    Column::new()
                        .push(text(name).size(14))
                        .push(text(desc).size(10)),
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
