use iced::widget::{Column, Container, Row, button, text};
use iced::{Element, Length};

use crate::circular::Circular;
use crate::message::{Message, QuickUpdateStatus};
use crate::state::{ProtonupGui, warning_button_style};

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    match &state.quick_update_status {
        QuickUpdateStatus::Checking => Column::new()
            .spacing(20)
            .push(text("Checking for updates...").size(16))
            .push(
                Container::new(Circular::new().size(40.0).bar_height(4.0))
                    .center_x(Length::Fill)
                    .padding(10),
            )
            .into(),
        QuickUpdateStatus::AllUpToDate(tool_names) => {
            let mut column = Column::new().spacing(15);

            column = column.push(
                text("✓ Tools are up to date.")
                    .size(16)
                    .color([0.3, 1.0, 0.3]),
            );

            column = column.push(text("The following tools are already installed:").size(14));

            for tool_name in tool_names {
                column = column.push(
                    Row::new()
                        .spacing(10)
                        .push(text("•").size(14))
                        .push(text(tool_name).size(14)),
                );
            }

            column = column.push(iced::widget::space::vertical().height(Length::Fixed(20.0)));

            column = column.push(
                button(text("Force Reinstallation").size(14))
                    .on_press(Message::ForceReinstall)
                    .padding(10)
                    .style(warning_button_style()),
            );

            column = column.push(
                button(text("Back to Main Menu").size(14))
                    .on_press(Message::BackToInitial)
                    .padding(10),
            );

            column.into()
        }
        QuickUpdateStatus::InProgress => Column::new()
            .spacing(10)
            .push(text("Quick Update in progress...").size(14))
            .into(),
        QuickUpdateStatus::Complete => Column::new()
            .spacing(10)
            .push(text("Quick Update complete.").size(14))
            .into(),
        QuickUpdateStatus::Idle => Column::new()
            .spacing(10)
            .push(text("Quick Update ready.").size(14))
            .into(),
    }
}
