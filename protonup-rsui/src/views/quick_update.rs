use iced::widget::{Column, Container, Row, button, scrollable, text};
use iced::{Element, Length};

use crate::circular::Circular;
use crate::message::{Focusable, Message, Pane, QuickUpdateStatus};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};
use crate::views::installed_versions;

use super::focus_style::{self, Highlight};

/// Focusable options of the Quick Update main pane.
///
/// Only the "see all versions" links and the installed-versions `Close` button
/// are interactive; the checking/in-progress/complete states have none.
pub(crate) fn main_focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    let QuickUpdateStatus::AllUpToDate(installed_tools) = &state.quick_update_status else {
        return Vec::new();
    };

    let mut rows: Vec<Vec<Focusable>> = installed_tools
        .iter()
        .filter(|installed| installed.latest.is_some() && installed.other_count() > 0)
        .map(|installed| {
            vec![Focusable::new(Message::ToggleInstalledVersions(Box::new(
                Some(installed.clone()),
            )))]
        })
        .collect();

    if state.show_all_versions.is_some() {
        rows.push(vec![Focusable::new(Message::ToggleInstalledVersions(
            Box::new(None),
        ))]);
    }

    rows
}

pub(crate) fn actions(_state: &ProtonupGui) -> Vec<Action> {
    vec![
        Action::new("Force Reinstallation", Message::ForceReinstall).warning(),
        Action::new("Back to Main Menu", Message::BackToInitial),
    ]
}

/// Quick Update's primary action: force reinstall when everything is
/// up to date.
pub(crate) fn primary_action(state: &ProtonupGui) -> Option<Message> {
    matches!(state.quick_update_status, QuickUpdateStatus::AllUpToDate(_))
        .then_some(Message::ForceReinstall)
}

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

            let mut link_index = 0usize;

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
                        let highlight = Highlight::from(focus_style::is_focused(
                            state,
                            Pane::Main,
                            link_index,
                            0,
                        ));
                        link_index += 1;

                        column = column.push(
                            Row::new().spacing(10).push(
                                iced::widget::space::horizontal().width(Length::Fixed(22.0)),
                            ).push(
                                focus_style::focus_frame(
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
                                    highlight,
                                ),
                            ),
                        );
                    }
                }
            }

            if state.show_all_versions.is_some() {
                let close_highlight =
                    Highlight::from(focus_style::is_focused(state, Pane::Main, link_index, 0));
                column = column.push(focus_style::focus_frame(
                    button(text("Hide installed versions").size(12))
                        .on_press(Message::ToggleInstalledVersions(Box::new(None)))
                        .padding(5),
                    close_highlight,
                ));
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

            action_bar::with_actions(content, actions(state), |col| {
                focus_style::action_focused(state, col)
            })
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
