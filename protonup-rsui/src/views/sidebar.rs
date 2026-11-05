use iced::widget::{Column, Container, button, container, image, rule, scrollable, text};
use iced::{ContentFit, Element, Length};

use crate::message::{AppMode, Focusable, Message, Pane, QuickUpdateStatus, SelectionStep};
use crate::state::{ProtonupGui, warning_button_style};

use super::focus_style::{self, Highlight};

mod circular_widget {
    pub use crate::circular::Circular;
}

/// Height reserved by the sidebar logo, in logical pixels.
const LOGO_HEIGHT: f32 = 120.0;

/// One sidebar entry: label, message and whether it can be activated.
struct SidebarEntry {
    label: &'static str,
    message: Message,
    enabled: bool,
    warning: bool,
    /// Rendered below the scrollable so it stays visible.
    pinned: bool,
}

/// The sidebar entries for the current state, in rendering order (excluding
/// the always-present `Close` button which is appended last).
fn entries(state: &ProtonupGui) -> Vec<SidebarEntry> {
    let is_downloading = state.download_started
        && state.selection_step == SelectionStep::Downloading
        && state.download_complete.is_none();

    let mk = |label: &'static str, message: Message, disabled_for: bool| SidebarEntry {
        label,
        message,
        enabled: !(is_downloading || disabled_for),
        warning: false,
        pinned: false,
    };

    let mut entries = vec![
        mk(
            "Quick Update",
            Message::SelectQuickUpdate,
            state.app_mode == AppMode::QuickUpdate,
        ),
        mk(
            "Download for Steam",
            Message::SelectDownloadForSteam,
            state.app_mode == AppMode::DownloadForSteam,
        ),
        mk(
            "Download for Lutris",
            Message::SelectDownloadForLutris,
            state.app_mode == AppMode::DownloadForLutris,
        ),
        mk(
            "Download for Custom Location",
            Message::SelectDownloadForCustom,
            state.app_mode == AppMode::DownloadForCustom,
        ),
        mk(
            "Check What's New",
            Message::SelectCheckWhatsNew,
            state.app_mode == AppMode::CheckWhatsNew,
        ),
        mk(
            "Manage Existing Installations",
            Message::SelectManageInstallations,
            state.app_mode == AppMode::ManageInstallations,
        ),
    ];

    if is_downloading {
        entries.push(SidebarEntry {
            label: "Cancel",
            message: Message::Cancel,
            enabled: true,
            warning: true,
            pinned: false,
        });
    }

    entries.push(SidebarEntry {
        label: "Close",
        message: Message::CloseRequested,
        enabled: true,
        warning: true,
        pinned: true,
    });

    entries
}

/// Focusable sidebar options as a one-column grid (row = option index).
pub(crate) fn focusable_options(state: &ProtonupGui) -> Vec<Vec<Focusable>> {
    entries(state)
        .into_iter()
        .filter(|entry| entry.enabled)
        .map(|entry| vec![Focusable::new(entry.message)])
        .collect()
}

pub(crate) fn sidebar(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10).width(Length::Fill);
    let mut pinned = Column::new().spacing(10).width(Length::Fill);
    let mut header = Column::new().spacing(10).width(Length::Fill);

    let logo_handle = &state.logo_handle;
    header = header.push(
        Container::new(
            image(logo_handle)
                .width(Length::Fill)
                .height(Length::Fixed(LOGO_HEIGHT))
                .content_fit(ContentFit::Contain),
        )
        .center_x(Length::Fill)
        .padding(5),
    );

    header = header.push(rule::horizontal(1));

    let is_downloading = state.download_started
        && state.selection_step == SelectionStep::Downloading
        && state.download_complete.is_none();

    let is_complete = state.download_complete.is_some();

    let show_spinner = is_downloading
        && !matches!(
            state.quick_update_status,
            QuickUpdateStatus::Checking | QuickUpdateStatus::AllUpToDate(_)
        );

    if show_spinner {
        header = header.push(
            Container::new(circular_widget::Circular::new().size(40.0).bar_height(4.0))
                .center_x(Length::Fill)
                .padding(10),
        );

        header = header
            .push(Container::new(text("Download in progress...").size(12)).center_x(Length::Fill));
    }

    if is_complete {
        header = header.push(
            Container::new(text("Completed ✅").size(14))
                .center_x(Length::Fill)
                .padding(10),
        );
    }

    // Focus index mirrors `focusable_options`, which skips disabled entries.
    let mut focus_index = 0usize;

    for entry in entries(state) {
        let highlight = if entry.enabled {
            let highlight = Highlight::from(focus_style::is_focused(
                state,
                Pane::Sidebar,
                focus_index,
                0,
            ));
            focus_index += 1;
            highlight
        } else {
            Highlight::None
        };

        let label = Container::new(text(entry.label).size(14))
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(10);

        // The pinned `Close` entry is rendered outside the scrollable, so it
        // stays reachable when the list is taller than the viewport.
        if entry.pinned {
            pinned = pinned.push(focus_style::focus_frame(
                button(label)
                    .on_press(entry.message)
                    .width(Length::Fill)
                    .style(warning_button_style()),
                highlight,
            ));
            continue;
        }

        let styled = if entry.warning && entry.enabled {
            button(label)
                .on_press(entry.message)
                .width(Length::Fill)
                .style(warning_button_style())
        } else if entry.enabled {
            button(label).on_press(entry.message).width(Length::Fill)
        } else {
            button(label).width(Length::Fill)
        };

        column = column.push(focus_style::focus_frame(styled, highlight));
    }

    let scrollable = scrollable(column)
        .id(super::scroll_id::sidebar())
        .width(Length::Fill)
        .height(Length::Fill);

    // The logo/status header sits above the scrollable so a short viewport
    // never has to fit it alongside the (scrollable) navigation entries. The
    // pinned `Close`/`Cancel` bar sits below it and therefore always shows.
    let content = Column::new()
        .spacing(10)
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .push(header)
        .push(scrollable)
        .push(pinned);

    container(content)
        .width(220)
        .height(Length::Fill)
        .style(focus_style::panel_style())
        .into()
}
