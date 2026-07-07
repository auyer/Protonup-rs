use iced::widget::{Column, center, container, text};
use iced::{Element, Fill};

use crate::message::{AppMode, GuiMode, Message, QuickUpdateStatus, SelectionStep};
use crate::state::ProtonupGui;

use super::{
    architecture_selection, confirm_reinstall, custom_location, download_progress,
    manage_installations, quick_update, tool_selection, version_selection,
};

pub(crate) fn view_main_content(state: &ProtonupGui) -> Element<'_, Message> {
    let content: Element<Message> = {
        if state.app_mode == AppMode::None {
            container(center(text("⬅️ Choose your option").size(18)))
                .width(Fill)
                .height(Fill)
                .into()
        } else if state.download_started
            && state.selection_step == SelectionStep::Downloading
            && !matches!(
                state.quick_update_status,
                QuickUpdateStatus::Checking | QuickUpdateStatus::AllUpToDate(_)
            )
        {
            Column::new()
                .spacing(10)
                .push(download_progress::view(state))
                .into()
        } else {
            match &state.mode {
                GuiMode::QuickUpdate => quick_update::view(state),
                GuiMode::DownloadForSteam
                | GuiMode::DownloadForLutris
                | GuiMode::DownloadForCustom => view_selection_flow(state),
                GuiMode::ManageInstallations => manage_installations::view(state),
                _ => container(center(text("⬅️ Choose your option").size(18)))
                    .width(Fill)
                    .height(Fill)
                    .into(),
            }
        }
    };

    container(content)
        .padding(20)
        .width(Fill)
        .height(Fill)
        .into()
}

fn view_selection_flow(state: &ProtonupGui) -> Element<'_, Message> {
    if state.mode == GuiMode::DownloadForCustom && state.selection_step == SelectionStep::Initial {
        return custom_location::view(state);
    }

    match &state.selection_step {
        SelectionStep::Initial => text("Initializing...").size(14).into(),
        SelectionStep::SelectingTools => tool_selection::view(state),
        SelectionStep::SelectingVersions => version_selection::view(state),
        SelectionStep::SelectingArchitecture => architecture_selection::view(state),
        SelectionStep::ConfirmReinstall => confirm_reinstall::view(state),
        SelectionStep::Downloading => text("Preparing downloads...").size(14).into(),
    }
}
