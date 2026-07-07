use iced::widget::{Column, Container, button, container, image, rule, text};
use iced::{ContentFit, Element, Length};

use crate::message::{AppMode, Message, QuickUpdateStatus, SelectionStep};
use crate::state::{ProtonupGui, warning_button_style};

mod circular_widget {
    pub use crate::circular::Circular;
}

pub(crate) fn sidebar(state: &ProtonupGui) -> Element<'_, Message> {
    let mut column = Column::new().spacing(10).padding(10).width(220);

    let logo_handle = &state.logo_handle;
    column = column.push(
        Container::new(
            image(logo_handle)
                .width(180)
                .height(Length::Fixed(180.0))
                .content_fit(ContentFit::Contain),
        )
        .center_x(Length::Fill)
        .padding(5),
    );

    column = column.push(rule::horizontal(1));

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
        column = column.push(
            Container::new(circular_widget::Circular::new().size(40.0).bar_height(4.0))
                .center_x(Length::Fill)
                .padding(10),
        );

        column = column
            .push(Container::new(text("Download in progress...").size(12)).center_x(Length::Fill));
    }

    if is_complete {
        column = column.push(
            Container::new(text("Completed ✅").size(14))
                .center_x(Length::Fill)
                .padding(10),
        );
    }

    let quick_update_disabled = is_downloading || state.app_mode == AppMode::QuickUpdate;
    column = column.push(if quick_update_disabled {
        button(text("Quick Update").size(14))
            .padding(10)
            .width(Length::Fill)
    } else {
        button(text("Quick Update").size(14))
            .on_press(Message::SelectQuickUpdate)
            .padding(10)
            .width(Length::Fill)
    });

    let steam_disabled = is_downloading || state.app_mode == AppMode::DownloadForSteam;
    column = column.push(if steam_disabled {
        button(text("Download for Steam").size(14))
            .padding(10)
            .width(Length::Fill)
    } else {
        button(text("Download for Steam").size(14))
            .on_press(Message::SelectDownloadForSteam)
            .padding(10)
            .width(Length::Fill)
    });

    let lutris_disabled = is_downloading || state.app_mode == AppMode::DownloadForLutris;
    column = column.push(if lutris_disabled {
        button(text("Download for Lutris").size(14))
            .padding(10)
            .width(Length::Fill)
    } else {
        button(text("Download for Lutris").size(14))
            .on_press(Message::SelectDownloadForLutris)
            .padding(10)
            .width(Length::Fill)
    });

    let custom_disabled = is_downloading || state.app_mode == AppMode::DownloadForCustom;
    column = column.push(if custom_disabled {
        button(text("Download for Custom Location").size(14))
            .padding(10)
            .width(Length::Fill)
    } else {
        button(text("Download for Custom Location").size(14))
            .on_press(Message::SelectDownloadForCustom)
            .padding(10)
            .width(Length::Fill)
    });

    let manage_disabled = is_downloading || state.app_mode == AppMode::ManageInstallations;
    column = column.push(if manage_disabled {
        button(text("Manage Existing Installations").size(14))
            .padding(10)
            .width(Length::Fill)
    } else {
        button(text("Manage Existing Installations").size(14))
            .on_press(Message::SelectManageInstallations)
            .padding(10)
            .width(Length::Fill)
    });

    if is_downloading {
        column = column.push(
            button(text("Cancel").size(14))
                .on_press(Message::Cancel)
                .padding(10)
                .width(Length::Fill)
                .style(warning_button_style()),
        );
    }

    column = column.push(iced::widget::space::vertical());

    column = column.push(
        button(text("Close").size(14))
            .on_press(Message::CloseRequested)
            .padding(10)
            .width(Length::Fill)
            .style(warning_button_style()),
    );

    container(column).style(container::rounded_box).into()
}
