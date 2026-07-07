use iced::Task;
use std::collections::HashSet;

use libprotonup::apps::{App, AppInstallations, list_installed_apps};
use libprotonup::sources::CompatTool;

use crate::download::{self, DownloadPhase};
use crate::download_task::{self, DownloadError, DownloadUpdate};
use crate::message::{
    AppInstallationView, AppMode, GuiMode, Message, QuickUpdateStatus, SelectionStep, ToolDownload,
};
use crate::state::ProtonupGui;

pub(crate) fn handle(state: &mut ProtonupGui, message: Message) -> Task<Message> {
    match message {
        Message::ScanApps => Task::perform(list_installed_apps(), Message::AppsScanned),

        Message::AppsScanned(apps) => {
            state.detected_apps = apps;
            state.scan_complete = true;
            if state.detected_apps.is_empty() {
                state.global_status = "No compatible apps detected".to_string();
            } else {
                state.global_status = format!(
                    "Detected: {}",
                    state
                        .detected_apps
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            Task::none()
        }

        Message::SelectQuickUpdate => {
            state.app_mode = AppMode::QuickUpdate;
            state.mode = GuiMode::QuickUpdate;
            state.selection_step = SelectionStep::Downloading;
            state.download_started = true;
            state.global_progress = 0.0;
            state.download_complete = None;
            state.global_status = "Checking for updates...".to_string();
            state.quick_update_status = QuickUpdateStatus::Checking;

            state.tools.clear();

            Task::perform(
                ProtonupGui::check_quick_update_installed(state.detected_apps.clone()),
                Message::QuickUpdateChecked,
            )
        }

        Message::SelectDownloadForSteam => {
            state.app_mode = AppMode::DownloadForSteam;
            state.mode = GuiMode::DownloadForSteam;
            state.selection_step = SelectionStep::SelectingTools;
            state.global_status = "Detecting Steam installation...".to_string();
            Task::perform(
                ProtonupGui::detect_app_and_fetch_tools(App::Steam),
                |result| match result {
                    Ok((app_inst, _tools)) => Message::AppInstallationDetected(app_inst),
                    Err(e) => Message::SelectionError(e),
                },
            )
        }

        Message::SelectDownloadForLutris => {
            state.app_mode = AppMode::DownloadForLutris;
            state.mode = GuiMode::DownloadForLutris;
            state.selection_step = SelectionStep::SelectingTools;
            state.global_status = "Detecting Lutris installation...".to_string();
            Task::perform(
                ProtonupGui::detect_app_and_fetch_tools(App::Lutris),
                |result| match result {
                    Ok((app_inst, _tools)) => Message::AppInstallationDetected(app_inst),
                    Err(e) => Message::SelectionError(e),
                },
            )
        }

        Message::AppInstallationDetected(app_inst) => {
            state.app_installation = Some(app_inst.clone());
            let tools = CompatTool::sources_for_app(&app_inst.as_app());
            state.available_tools = tools;
            state.selected_tool_indices.clear();
            state.selection_step = SelectionStep::SelectingTools;
            Task::none()
        }

        Message::ToolSelected(index) => {
            state.selected_tool_indices.clear();
            state.selected_tool_indices.push(index);
            Task::none()
        }

        Message::ToolSelectionConfirmed => {
            if state.mode == GuiMode::DownloadForCustom && state.app_installation.is_none() {
                if state.custom_path_input.is_empty() {
                    state.path_error = Some("Please enter a valid path".to_string());
                    return Task::none();
                }

                state.app_installation = Some(AppInstallations::new_custom_app_install(
                    state.custom_path_input.clone(),
                ));

                state.available_tools = libprotonup::sources::CompatTools.clone();
                state.selected_tool_indices.clear();
                state.selection_step = SelectionStep::SelectingTools;
                state.global_status = "Select tools to install".to_string();
                return Task::none();
            }

            if state.selected_tool_indices.is_empty() {
                state.global_status = "Please select at least one tool".to_string();
                return Task::none();
            }

            let tool = state.available_tools[state.selected_tool_indices[0]].clone();
            state.selected_tool = Some(tool.clone());
            state.selection_step = SelectionStep::SelectingVersions;
            state.global_status = format!("Fetching releases for {}...", tool.name);

            Task::perform(download::fetch_releases(tool), Message::VersionsFetched)
        }

        Message::VersionsFetched(releases) => {
            state.available_versions = releases;
            state.selected_version_indices.clear();
            if !state.available_versions.is_empty() {
                state.selected_version_indices.push(0);
            }

            state.has_variant_tools = state.selected_tool_indices.iter().any(|&idx| {
                state
                    .available_tools
                    .get(idx)
                    .is_some_and(|t| t.has_multiple_asset_variations)
            });

            if state.has_variant_tools {
                state.selection_step = SelectionStep::SelectingArchitecture;
                state.selected_arch_variant = Some(2);
            } else {
                state.selection_step = SelectionStep::SelectingVersions;
            }
            Task::none()
        }

        Message::ToggleVersion(index) => {
            if let Some(pos) = state
                .selected_version_indices
                .iter()
                .position(|&i| i == index)
            {
                state.selected_version_indices.remove(pos);
            } else {
                state.selected_version_indices.push(index);
            }
            Task::none()
        }

        Message::SelectArchitecture(variant_code) => {
            state.selected_arch_variant = Some(variant_code);
            Task::none()
        }

        Message::StartSelectedDownloads => {
            if state.selected_tool_indices.is_empty() || state.selected_version_indices.is_empty() {
                state.global_status = "Please select tools and versions".to_string();
                return Task::none();
            }

            let mut tools_and_versions = Vec::new();
            for &tool_idx in &state.selected_tool_indices {
                let tool = state.available_tools[tool_idx].clone();
                let versions: Vec<libprotonup::downloads::Release> = state
                    .selected_version_indices
                    .iter()
                    .map(|&v_idx| state.available_versions[v_idx].clone())
                    .collect();
                tools_and_versions.push((tool, versions));
            }

            let app_inst = state.app_installation.clone().unwrap();

            Task::perform(
                ProtonupGui::check_already_installed(app_inst, tools_and_versions),
                Message::AlreadyInstalledChecked,
            )
        }

        Message::AlreadyInstalledChecked(already_installed) => {
            state.already_installed_tools = already_installed;
            state.force_reinstall_indices.clear();

            if state.already_installed_tools.is_empty() {
                state.start_downloads(HashSet::new())
            } else {
                state.selection_step = SelectionStep::ConfirmReinstall;
                state.global_status = format!(
                    "{} tool(s) already installed. Select which to reinstall.",
                    state.already_installed_tools.len()
                );
                Task::none()
            }
        }

        Message::QuickUpdateChecked(results) => {
            if state.app_mode != AppMode::QuickUpdate {
                return Task::none();
            }

            let all_installed = results.iter().all(|(_, installed)| *installed);
            if all_installed && !results.is_empty() {
                let tool_names: Vec<String> = results.into_iter().map(|(name, _)| name).collect();
                state.quick_update_status = QuickUpdateStatus::AllUpToDate(tool_names);
                state.global_status = "Tools are up to date.".to_string();
                Task::none()
            } else {
                state.quick_update_status = QuickUpdateStatus::InProgress;
                state.global_status = "Starting Quick Update...".to_string();

                state.tools.clear();

                let (task, handle) = download_task::run_quick_update(false);
                state.download_handle = Some(handle);

                task.map(Message::DownloadUpdate)
            }
        }

        Message::ForceReinstall => {
            state.quick_update_status = QuickUpdateStatus::InProgress;
            state.global_status = "Force reinstalling tools...".to_string();

            state.tools.clear();

            let (task, handle) = download_task::run_quick_update(true);
            state.download_handle = Some(handle);

            task.map(Message::DownloadUpdate)
        }

        Message::ToggleReinstall(index) => {
            if let Some(pos) = state
                .force_reinstall_indices
                .iter()
                .position(|&i| i == index)
            {
                state.force_reinstall_indices.remove(pos);
            } else {
                state.force_reinstall_indices.push(index);
            }
            Task::none()
        }

        Message::ConfirmReinstallSelection => {
            let force_reinstall_names: HashSet<String> = state
                .force_reinstall_indices
                .iter()
                .filter_map(|&i| state.already_installed_tools.get(i))
                .map(|t| t.name.clone())
                .collect();
            state.start_downloads(force_reinstall_names)
        }

        Message::DownloadUpdate(update) => match update {
            DownloadUpdate::ToolProgress(progress) => {
                if let Some(tool) = state
                    .tools
                    .iter_mut()
                    .find(|t| t.name == progress.tool_name)
                {
                    tool.update_from_progress(&progress);
                } else {
                    let mut tool = ToolDownload::new(progress.tool_name.clone());
                    tool.update_from_progress(&progress);
                    state.tools.push(tool);
                }
                Task::none()
            }
            DownloadUpdate::GlobalProgress(progress) => {
                state.global_phase = progress.phase;
                state.global_status = progress.status_message;
                state.global_progress = progress.percent;
                Task::none()
            }
            DownloadUpdate::Finished(result) => {
                state.download_handle = None;

                match result {
                    Ok(versions) => {
                        state.global_progress = 100.0;
                        state.global_phase = DownloadPhase::Complete;
                        state.global_status =
                            format!("✓ Success! Installed {} tools.", versions.len());
                        state.download_complete = Some(Ok(versions));
                        if state.app_mode == AppMode::QuickUpdate {
                            state.quick_update_status = QuickUpdateStatus::Complete;
                        }
                    }
                    Err(e) => {
                        let DownloadError::IoError(error_msg) = e;
                        state.global_phase = DownloadPhase::Error;
                        state.global_status = format!("✗ Error: {}", error_msg);
                        state.download_complete = Some(Err(error_msg));
                    }
                }
                Task::none()
            }
        },

        Message::Cancel => {
            if let Some(handle) = state.download_handle.take() {
                handle.abort();
            }
            state.reset_to_initial();
            state.app_mode = AppMode::None;
            Task::none()
        }

        Message::BackToInitial => {
            state.reset_to_initial();
            state.app_mode = AppMode::None;
            Task::none()
        }

        Message::BackToToolSelection => {
            state.selection_step = SelectionStep::SelectingTools;
            state.selected_version_indices.clear();
            state.available_versions.clear();
            state.selected_tool = None;
            Task::none()
        }

        Message::SelectionError(e) => {
            state.global_status = format!("Error: {}", e);
            state.selection_step = SelectionStep::Initial;
            Task::none()
        }

        Message::TickSpinner => Task::none(),

        Message::CloseRequested => {
            std::process::exit(0);
        }

        Message::SelectDownloadForCustom => {
            state.app_mode = AppMode::DownloadForCustom;
            state.mode = GuiMode::DownloadForCustom;
            state.selection_step = SelectionStep::Initial;
            state.custom_path_input = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            state.path_error = None;
            Task::none()
        }

        Message::CustomPathInput(path) => {
            state.custom_path_input = path;
            state.path_error = None;
            Task::none()
        }

        Message::OpenFolderPicker => Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .pick_folder()
                    .await
                    .map(|handle| handle.path().to_path_buf())
            },
            Message::FolderPicked,
        ),

        Message::FolderPicked(Some(path)) => {
            state.custom_path_input = path.to_string_lossy().to_string();
            state.path_error = None;
            Task::none()
        }

        Message::FolderPicked(None) => Task::none(),

        Message::SelectManageInstallations => {
            state.app_mode = AppMode::ManageInstallations;
            state.mode = GuiMode::ManageInstallations;
            state.manage_status = "Scanning for installed versions...".to_string();
            state.manage_error = None;

            state.app_installations_views = libprotonup::apps::APP_INSTALLATIONS_VARIANTS
                .iter()
                .map(|app| AppInstallationView {
                    app: app.clone(),
                    selected: true,
                    versions: vec![],
                    loading: true,
                })
                .collect();

            Task::perform(
                ProtonupGui::scan_all_installed_versions(),
                Message::VersionsScanned,
            )
        }

        Message::AppSelectionToggled(index) => {
            if let Some(view) = state.app_installations_views.get_mut(index) {
                view.selected = !view.selected;
            }
            Task::none()
        }

        Message::VersionToggled(app_index, version_index) => {
            if let Some(view) = state.app_installations_views.get_mut(app_index)
                && let Some(version) = view.versions.get_mut(version_index)
            {
                version.selected_for_deletion = !version.selected_for_deletion;
            }
            Task::none()
        }

        Message::DeleteSelectedVersions => {
            let selected: Vec<(usize, usize, std::path::PathBuf)> = state
                .app_installations_views
                .iter()
                .enumerate()
                .flat_map(|(app_idx, view)| {
                    view.versions
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.selected_for_deletion)
                        .map(move |(ver_idx, v)| (app_idx, ver_idx, v.path.clone()))
                        .collect::<Vec<_>>()
                })
                .collect();

            if selected.is_empty() {
                state.manage_status = "No versions selected for deletion".to_string();
                return Task::none();
            }

            state.manage_status = format!("Deleting {} version(s)...", selected.len());

            Task::perform(
                ProtonupGui::delete_versions(selected),
                Message::DeleteCompleted,
            )
        }

        Message::DeleteCompleted(result) => match result {
            Ok(deleted) => {
                state.manage_status = format!("✓ Deleted {} version(s)", deleted.len());
                Task::perform(
                    ProtonupGui::scan_all_installed_versions(),
                    Message::VersionsScanned,
                )
            }
            Err(e) => {
                state.manage_error = Some(e);
                state.manage_status = "Error deleting versions".to_string();
                Task::none()
            }
        },

        Message::VersionsScanned(versions) => {
            for (i, view) in state.app_installations_views.iter_mut().enumerate() {
                if let Some((_, vers)) = versions.get(i) {
                    view.versions = vers
                        .iter()
                        .map(|(parent_path, name)| crate::message::InstalledVersion {
                            name: name.clone(),
                            path: parent_path.join(name),
                            selected_for_deletion: false,
                        })
                        .collect();
                    view.loading = false;
                }
            }

            let total_versions: usize = state
                .app_installations_views
                .iter()
                .map(|v| v.versions.len())
                .sum();
            state.manage_status = format!(
                "Found {} version(s) across {} app(s)",
                total_versions,
                state.app_installations_views.len()
            );
            Task::none()
        }
    }
}
