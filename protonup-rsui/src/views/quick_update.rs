use iced::widget::{Column, Container, Row, button, scrollable, text};
use iced::{Element, Length};

use crate::circular::Circular;
use crate::message::{Message, QuickUpdateStatus};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};
use crate::views::installed_versions;

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
        QuickUpdateStatus::AllUpToDate(installed_tools) => {
            let mut column = Column::new().spacing(15);

            column = column.push(
                text("✓ Tools are up to date.")
                    .size(16)
                    .color([0.3, 1.0, 0.3]),
            );

            column = column.push(text("The following tools are already installed:").size(14));

            for installed in installed_tools {
                column = column.push(text(format!("{}", installed.app)).size(14));

                if let Some(latest) = &installed.latest {
                    column = column.push(
                        Row::new()
                            .spacing(10)
                            .push(text("•").size(14))
                            .push(text(format!("Latest: {} {}", installed.tool, latest)).size(14)),
                    );

                    let other_count = installed.other_count();
                    if other_count > 0 {
                        let info = installed.clone();
                        column = column.push(
                            Row::new().spacing(10).push(
                                iced::widget::space::horizontal().width(Length::Fixed(22.0)),
                            ).push(
                                button(
                                    text(format!(
                                        "({other_count} other version{} installed — click to see all)",
                                        if other_count == 1 { "" } else { "s" }
                                    ))
                                    .size(13)
                                    .color([0.3, 0.5, 0.9]),
                                )
                                .on_press(Message::ToggleInstalledVersions(Box::new(Some(info))))
                                .style(iced::widget::button::text)
                                .padding([1.0, 4.0]),
                            ),
                        );
                    }
                }
            }

            let content: Element<'_, Message> = {
                let left_element: Element<_> = scrollable(column).into();

                if let Some(info) = &state.show_all_versions {
                    let right = Container::new(installed_versions::view(info))
                        .width(Length::FillPortion(2))
                        .padding(10);

                    let left_portion = Container::new(left_element).width(Length::FillPortion(1));

                    Row::new().push(left_portion).push(right).into()
                } else {
                    left_element
                }
            };

            action_bar::with_actions(
                content,
                [
                    Action::new("Force Reinstallation", Message::ForceReinstall).warning(),
                    Action::new("Back to Main Menu", Message::BackToInitial),
                ],
            )
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
