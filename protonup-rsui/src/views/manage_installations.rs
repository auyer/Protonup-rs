use iced::widget::{Column, Row, button, checkbox, container, rule, scrollable, text};
use iced::{Center, Element, Fill, Length};

use crate::message::Message;
use crate::state::ProtonupGui;

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut main_column = Column::new().spacing(20);

    main_column = main_column.push(text(&state.manage_status).size(14));

    if let Some(ref error) = state.manage_error {
        main_column = main_column.push(text(error).size(12).color([1.0, 0.3, 0.3]));
    }

    let mut row = Row::new().spacing(20);

    for (app_idx, view) in state.app_installations_views.iter().enumerate() {
        let mut col = Column::new().spacing(10).width(Length::Fill);

        col = col.push(
            Row::new()
                .spacing(10)
                .align_y(Center)
                .push(
                    checkbox(view.selected)
                        .on_toggle(move |_| Message::AppSelectionToggled(app_idx)),
                )
                .push(text(format!("{}", view.app)).size(14)),
        );

        col = col.push(rule::horizontal(1));

        if view.loading {
            col = col.push(text("Scanning...").size(12));
        } else if view.versions.is_empty() {
            col = col.push(text("No versions found").size(12).color([0.6, 0.6, 0.6]));
        } else {
            for (ver_idx, version) in view.versions.iter().enumerate() {
                col = col.push(
                    Row::new()
                        .spacing(10)
                        .align_y(Center)
                        .push(
                            checkbox(version.selected_for_deletion)
                                .on_toggle(move |_| Message::VersionToggled(app_idx, ver_idx)),
                        )
                        .push(text(&version.name).size(12)),
                );
            }
        }

        row = row.push(container(col).padding(10).style(container::rounded_box));
    }

    main_column = main_column.push(row);

    let has_selections = state
        .app_installations_views
        .iter()
        .any(|v| v.versions.iter().any(|ver| ver.selected_for_deletion));

    main_column = main_column.push(if has_selections {
        button(text("Delete Selected").size(14))
            .on_press(Message::DeleteSelectedVersions)
            .padding(10)
    } else {
        button(text("Delete Selected").size(14)).padding(10)
    });

    main_column = main_column.push(
        button(text("Back to Main Menu").size(14))
            .on_press(Message::BackToInitial)
            .padding(10),
    );

    container(scrollable(main_column))
        .padding(20)
        .width(Fill)
        .height(Fill)
        .into()
}
