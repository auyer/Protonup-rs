use iced::widget::{Column, Row, checkbox, container, rule, scrollable, text};
use iced::{Center, Element, Length};

use crate::message::{AppInstallationView, Focusable, Message, Pane};
use crate::state::ProtonupGui;
use crate::views::action_bar::{self, Action};

use super::focus_style::{self, Highlight};

/// Splits the app views into the ones that have installed versions and the
/// ones that have none, preserving the original order within each group.
pub(crate) fn partition_views(
    views: &[AppInstallationView],
) -> (Vec<&AppInstallationView>, Vec<&AppInstallationView>) {
    views.iter().partition(|view| !view.versions.is_empty())
}

/// Focusable options of the manage pane: one checkbox per installed version,
/// laid out as apps-as-columns × versions-as-rows (matching the rendered grid).
pub(crate) fn main_focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    let (populated, _empty) = partition_views(&state.app_installations_views);

    let max_versions = populated
        .iter()
        .map(|view| view.versions.len())
        .max()
        .unwrap_or(0);

    (0..max_versions)
        .map(|row| {
            populated
                .iter()
                .filter_map(|view| {
                    let app_idx = state
                        .app_installations_views
                        .iter()
                        .position(|candidate| std::ptr::eq(candidate, *view))?;
                    view.versions
                        .get(row)
                        .map(|_| Focusable::new(Message::VersionToggled(app_idx, row)))
                })
                .collect::<Vec<_>>()
        })
        .filter(|row| !row.is_empty())
        .collect()
}

pub(crate) fn actions(state: &ProtonupGui) -> Vec<Action> {
    let has_selections = state
        .app_installations_views
        .iter()
        .any(|v| v.versions.iter().any(|ver| ver.selected_for_deletion));

    let delete_action = if has_selections {
        Action::new("Delete Selected", Message::DeleteSelectedVersions)
    } else {
        Action::disabled("Delete Selected")
    };

    vec![
        delete_action,
        Action::new("Back to Main Menu", Message::BackToInitial),
    ]
}

/// Manage installations' primary action: delete the current selection.
pub(crate) fn primary_action(state: &ProtonupGui) -> Option<Message> {
    state
        .app_installations_views
        .iter()
        .any(|v| v.versions.iter().any(|ver| ver.selected_for_deletion))
        .then_some(Message::DeleteSelectedVersions)
}

pub(crate) fn view(state: &ProtonupGui) -> Element<'_, Message> {
    let mut main_column: Column<'_, Message> = Column::new().spacing(20);

    main_column = main_column.push(text(&state.manage_status).size(14));

    if let Some(ref error) = state.manage_error {
        main_column = main_column.push(text(error).size(12).color([1.0, 0.3, 0.3]));
    }

    let (populated, empty) = partition_views(&state.app_installations_views);
    let mut row = Row::new().spacing(20);

    // Apps with installed tools first, each in its own column.
    for (col_rank, view) in populated.iter().enumerate() {
        let app_idx = state
            .app_installations_views
            .iter()
            .position(|candidate| std::ptr::eq(candidate, *view))
            .unwrap_or_default();

        let mut col = Column::new().spacing(10).width(Length::Fill);

        col = col.push(text(format!("{}", view.app)).size(14));

        col = col.push(rule::horizontal(1));

        for (ver_idx, version) in view.versions.iter().enumerate() {
            // The focus cursor's column is the populated-app rank, matching
            // the grid built by `main_focusable_options` — not `app_idx`.
            let highlight = Highlight::from(focus_style::is_focused(
                state,
                Pane::Main,
                ver_idx,
                col_rank,
            ));

            col = col.push(focus_style::option_row(
                Row::new()
                    .spacing(10)
                    .align_y(Center)
                    .push(
                        checkbox(version.selected_for_deletion)
                            .on_toggle(move |_| Message::VersionToggled(app_idx, ver_idx))
                            .style(focus_style::checkbox_style(highlight)),
                    )
                    .push(text(&version.name).size(12)),
                highlight,
                Some(Message::VersionToggled(app_idx, ver_idx)),
            ));
        }

        row = row.push(container(col).padding(10).style(container::rounded_box));
    }

    // Apps without installed tools are grouped into a single column at the end.
    if !empty.is_empty() {
        let mut col = Column::new().spacing(10).width(Length::Fill);
        col = col.push(text("No installed tools").size(14));
        col = col.push(rule::horizontal(1));

        for view in empty {
            if view.loading {
                col = col.push(text(format!("{} - Scanning...", view.app)).size(12));
            } else {
                col = col.push(
                    text(format!("{}", view.app))
                        .size(12)
                        .color([0.6, 0.6, 0.6]),
                );
            }
        }

        row = row.push(container(col).padding(10).style(container::rounded_box));
    }

    main_column = main_column.push(row);

    let content: Element<'_, Message> = scrollable(main_column).id(super::scroll_id::main()).into();

    action_bar::with_actions(content, actions(state), |col| {
        focus_style::action_focused(state, col)
    })
}
