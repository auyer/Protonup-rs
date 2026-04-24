use iced::task;
use iced::time;
use iced::widget::{
    button, center, checkbox, container, image, progress_bar, radio, rule, scrollable, space, text,
    text_input, Column, Container, Row,
};
use iced::window;
use iced::{Border, Center, Color, ContentFit, Element, Fill, Length, Subscription, Task};

use libprotonup::apps::{list_installed_apps, App, AppInstallations};
use libprotonup::downloads::Release;
use libprotonup::files;
use libprotonup::sources::CompatTool;

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

// This embeds the bytes directly into the binary
const LOGO_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/protonup--rs-logo.png"
));

mod circular;
mod easing;
use circular::Circular;

mod download;
mod download_task;
use download::DownloadPhase;
use download_task::{DownloadError, DownloadUpdate, ToolProgress};

#[cfg(test)]
mod gui_tests;

#[derive(Debug, Clone)]
enum Message {
    // Initial actions
    ScanApps,
    AppsScanned(Vec<AppInstallations>),

    // Mode selection
    SelectQuickUpdate,
    SelectDownloadForSteam,
    SelectDownloadForLutris,

    // Tool selection
    AppInstallationDetected(AppInstallations),
    ToolSelected(usize),
    ToolSelectionConfirmed,

    // Version selection
    VersionsFetched(Vec<Release>),
    ToggleVersion(usize),
    StartSelectedDownloads,

    // Architecture variant selection
    SelectArchitecture(u8),

    // Reinstall confirmation
    AlreadyInstalledChecked(Vec<ToolDownload>),
    ToggleReinstall(usize),
    ConfirmReinstallSelection,

    // Download progress
    DownloadUpdate(DownloadUpdate),

    // Quick update specific
    QuickUpdateChecked(Vec<(String, bool)>), // (tool_name, is_installed)
    ForceReinstall,

    // Navigation
    BackToInitial,
    BackToToolSelection,

    // Errors
    SelectionError(String),

    // Spinner animation
    TickSpinner,

    // Cancel download
    Cancel,

    // Layout
    CloseRequested,

    // Custom location flow
    SelectDownloadForCustom,
    CustomPathInput(String),
    OpenFolderPicker,
    FolderPicked(Option<PathBuf>),

    // Manage existing installations
    SelectManageInstallations,
    AppSelectionToggled(usize),
    VersionToggled(usize, usize),
    DeleteSelectedVersions,
    DeleteCompleted(Result<Vec<String>, String>),
    VersionsScanned(Vec<(AppInstallations, Vec<(PathBuf, String)>)>),
}

/// GUI mode - what the user is doing
#[derive(Debug, Clone, PartialEq, Default)]
enum GuiMode {
    #[default]
    Initial,
    QuickUpdate,
    DownloadForSteam,
    DownloadForLutris,
    DownloadForCustom,
    ManageInstallations,
}

/// Which action was selected in the sidebar
#[derive(Debug, Clone, PartialEq, Default)]
enum AppMode {
    #[default]
    None, // No action selected yet
    QuickUpdate,
    DownloadForSteam,
    DownloadForLutris,
    DownloadForCustom,
    ManageInstallations,
}

/// Current step in the selection flow
#[derive(Debug, Clone, PartialEq, Default)]
enum SelectionStep {
    #[default]
    Initial,
    SelectingTools,
    SelectingVersions,
    SelectingArchitecture, // NEW: Show architecture variant selection
    ConfirmReinstall,
    Downloading,
}

/// Tracks the state of a single tool download
#[derive(Debug, Clone)]
struct ToolDownload {
    name: String,
    app_target: String,
    phase: DownloadPhase,
    progress: f32,
    status: ToolStatus,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum ToolStatus {
    #[default]
    Pending,
    Downloading,
    Validating,
    Unpacking,
    _Complete,
    Error(String),
}

/// Quick update lifecycle state
#[derive(Debug, Clone, PartialEq, Default)]
enum QuickUpdateStatus {
    #[default]
    Idle,
    Checking,
    AllUpToDate(Vec<String>), // tool names that are up to date
    InProgress,
    Complete,
}

impl ToolDownload {
    fn new(name: String, app_target: String) -> Self {
        Self {
            name,
            app_target,
            phase: DownloadPhase::DetectingApps,
            progress: 0.0,
            status: ToolStatus::Pending,
        }
    }

    fn update_from_progress(&mut self, progress: &ToolProgress) {
        self.phase = progress.phase.clone();
        self.progress = progress.percent;

        match &self.phase {
            DownloadPhase::DetectingApps | DownloadPhase::FetchingReleases => {
                self.status = ToolStatus::Pending;
            }
            DownloadPhase::Downloading => {
                self.status = ToolStatus::Downloading;
            }
            DownloadPhase::Validating => {
                self.status = ToolStatus::Validating;
            }
            DownloadPhase::Unpacking => {
                self.status = ToolStatus::Unpacking;
            }
            DownloadPhase::Complete => {
                self.status = ToolStatus::_Complete;
            }
            DownloadPhase::Error => {
                self.status = ToolStatus::Error(progress.status_message.clone());
            }
        }
    }

    fn status_text(&self) -> String {
        match &self.status {
            ToolStatus::Pending => format!("{} - Waiting...", self.name),
            ToolStatus::Downloading => {
                format!("{} - Downloading... {:.1}%", self.name, self.progress)
            }
            ToolStatus::Validating => {
                format!("{} - Validating... {:.1}%", self.name, self.progress)
            }
            ToolStatus::Unpacking => format!("{} - Installing... {:.1}%", self.name, self.progress),
            ToolStatus::_Complete => format!("{} - ✓ Installed", self.name),
            ToolStatus::Error(msg) => format!("{} - ✗ Error: {}", self.name, msg),
        }
    }
}

/// Represents an installed compatibility tool version
#[derive(Debug, Clone)]
struct InstalledVersion {
    name: String,
    path: PathBuf,
    selected_for_deletion: bool,
}

/// Represents a single app installation view in the manage screen
#[derive(Debug)]
struct AppInstallationView {
    app: AppInstallations,
    selected: bool,
    versions: Vec<InstalledVersion>,
    loading: bool,
}

#[derive(Debug)]
struct ProtonupGui {
    // App detection
    detected_apps: Vec<AppInstallations>,
    scan_complete: bool,

    // Mode and selection state
    mode: GuiMode,
    selection_step: SelectionStep,

    // Tool selection
    available_tools: Vec<CompatTool>,
    selected_tool_indices: Vec<usize>,

    // Version selection
    selected_tool: Option<CompatTool>,
    available_versions: Vec<Release>,
    selected_version_indices: Vec<usize>,

    // Architecture variant selection
    selected_arch_variant: Option<u8>, // 1=x86_64, 2=v2, 3=v3, 4=v4
    has_variant_tools: bool,           // True if any selected tool has variants

    // App installation target
    app_installation: Option<AppInstallations>,

    // Reinstall confirmation state
    already_installed_tools: Vec<ToolDownload>,
    force_reinstall_indices: Vec<usize>,

    // Download state (shared with QuickUpdate)
    download_started: bool,
    tools: Vec<ToolDownload>,
    global_phase: DownloadPhase,
    global_status: String,
    global_progress: f32,
    download_complete: Option<Result<Vec<String>, String>>,

    // Cancel handle for aborting downloads
    download_handle: Option<task::Handle>,

    // Layout state
    app_mode: AppMode, // Track which action was selected

    // Custom location state
    custom_path_input: String,
    path_error: Option<String>,

    // Manage installations state
    app_installations_views: Vec<AppInstallationView>,
    manage_status: String,
    manage_error: Option<String>,

    // Quick update specific state
    quick_update_status: QuickUpdateStatus,

    // Persistent image handle (created once, reused across all view calls)
    logo_handle: image::Handle,
}

impl Default for ProtonupGui {
    fn default() -> Self {
        Self {
            detected_apps: Vec::new(),
            scan_complete: false,
            mode: GuiMode::default(),
            selection_step: SelectionStep::default(),
            available_tools: Vec::new(),
            selected_tool_indices: Vec::new(),
            selected_tool: None,
            available_versions: Vec::new(),
            selected_version_indices: Vec::new(),
            selected_arch_variant: None,
            has_variant_tools: false,
            app_installation: None,
            already_installed_tools: Vec::new(),
            force_reinstall_indices: Vec::new(),
            download_started: false,
            tools: Vec::new(),
            global_phase: DownloadPhase::default(),
            global_status: String::new(),
            global_progress: 0.0,
            download_complete: None,
            download_handle: None,
            app_mode: AppMode::default(),
            custom_path_input: String::new(),
            path_error: None,
            app_installations_views: Vec::new(),
            manage_status: String::new(),
            manage_error: None,
            quick_update_status: QuickUpdateStatus::default(),
            logo_handle: image::Handle::from_bytes(LOGO_BYTES),
        }
    }
}

/// Warning button style (yellow/orange for cautionary actions like Cancel/Close)
fn warning_button_style(
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    |theme, status| {
        let palette = theme.extended_palette();
        let warning_color = Color::from_rgb(0.6, 0.5, 0.2); // Yellow-orange warning color

        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.65, 0.0))),
                text_color: palette.background.base.text,
                border: Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.75, 0.55, 0.0),
                },
                shadow: Default::default(),
                snap: Default::default(),
            },
            iced::widget::button::Status::Active => iced::widget::button::Style {
                background: Some(iced::Background::Color(warning_color)),
                text_color: palette.background.base.text,
                border: Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.85, 0.65, 0.0),
                },
                shadow: Default::default(),
                snap: Default::default(),
            },
            iced::widget::button::Status::Pressed => iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.75, 0.55, 0.0))),
                text_color: palette.background.base.text,
                border: Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color::from_rgb(0.65, 0.45, 0.0),
                },
                shadow: Default::default(),
                snap: Default::default(),
            },
            iced::widget::button::Status::Disabled => iced::widget::button::Style {
                background: Some(iced::Background::Color(Color::from_rgba(
                    0.95, 0.75, 0.0, 0.5,
                ))),
                text_color: Color::from_rgba(0.5, 0.5, 0.5, 0.5),
                border: Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: Color::from_rgba(0.85, 0.65, 0.0, 0.5),
                },
                shadow: Default::default(),
                snap: Default::default(),
            },
        }
    }
}

impl ProtonupGui {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanApps => Task::perform(list_installed_apps(), Message::AppsScanned),

            Message::AppsScanned(apps) => {
                self.detected_apps = apps;
                self.scan_complete = true;
                if self.detected_apps.is_empty() {
                    self.global_status = "No compatible apps detected".to_string();
                } else {
                    self.global_status = format!(
                        "Detected: {}",
                        self.detected_apps
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                Task::none()
            }

            Message::SelectQuickUpdate => {
                self.app_mode = AppMode::QuickUpdate;
                self.mode = GuiMode::QuickUpdate;
                self.selection_step = SelectionStep::Downloading;
                self.download_started = true;
                self.global_progress = 0.0;
                self.download_complete = None;
                self.global_status = "Checking for updates...".to_string();
                self.quick_update_status = QuickUpdateStatus::Checking;

                // Clear tools - we'll populate after checking
                self.tools.clear();

                // Check if tools are already installed before starting download
                Task::perform(
                    Self::check_quick_update_installed(self.detected_apps.clone()),
                    Message::QuickUpdateChecked,
                )
            }

            Message::SelectDownloadForSteam => {
                self.app_mode = AppMode::DownloadForSteam;
                self.mode = GuiMode::DownloadForSteam;
                self.selection_step = SelectionStep::SelectingTools;
                self.global_status = "Detecting Steam installation...".to_string();
                Task::perform(
                    Self::detect_app_and_fetch_tools(App::Steam),
                    |result| match result {
                        Ok((app_inst, _tools)) => Message::AppInstallationDetected(app_inst),
                        Err(e) => Message::SelectionError(e),
                    },
                )
            }

            Message::SelectDownloadForLutris => {
                self.app_mode = AppMode::DownloadForLutris;
                self.mode = GuiMode::DownloadForLutris;
                self.selection_step = SelectionStep::SelectingTools;
                self.global_status = "Detecting Lutris installation...".to_string();
                Task::perform(
                    Self::detect_app_and_fetch_tools(App::Lutris),
                    |result| match result {
                        Ok((app_inst, _tools)) => Message::AppInstallationDetected(app_inst),
                        Err(e) => Message::SelectionError(e),
                    },
                )
            }

            Message::AppInstallationDetected(app_inst) => {
                self.app_installation = Some(app_inst.clone());
                let tools = CompatTool::sources_for_app(&app_inst.as_app());
                self.available_tools = tools;
                self.selected_tool_indices.clear();
                self.selection_step = SelectionStep::SelectingTools;
                Task::none()
            }

            Message::ToolSelected(index) => {
                // Replace selection with single tool (radio button behavior)
                self.selected_tool_indices.clear();
                self.selected_tool_indices.push(index);
                Task::none()
            }

            Message::ToolSelectionConfirmed => {
                // Handle custom location mode
                if self.mode == GuiMode::DownloadForCustom {
                    // If app_installation is not set, we're at the path selection step
                    if self.app_installation.is_none() {
                        // Validate path
                        if self.custom_path_input.is_empty() {
                            self.path_error = Some("Please enter a valid path".to_string());
                            return Task::none();
                        }

                        // Create custom app installation
                        self.app_installation = Some(AppInstallations::new_custom_app_install(
                            self.custom_path_input.clone(),
                        ));

                        // Fetch all available tools for custom location
                        self.available_tools = libprotonup::sources::CompatTools.clone();
                        self.selected_tool_indices.clear();
                        self.selection_step = SelectionStep::SelectingTools;
                        self.global_status = "Select tools to install".to_string();
                        return Task::none();
                    }

                    // If app_installation is already set, we're at tool selection - proceed normally
                    // Fall through to the normal tool selection logic below
                }

                if self.selected_tool_indices.is_empty() {
                    self.global_status = "Please select at least one tool".to_string();
                    return Task::none();
                }

                // Get the first selected tool for version selection
                let tool = self.available_tools[self.selected_tool_indices[0]].clone();
                self.selected_tool = Some(tool.clone());
                self.selection_step = SelectionStep::SelectingVersions;
                self.global_status = format!("Fetching releases for {}...", tool.name);

                Task::perform(download::fetch_releases(tool), Message::VersionsFetched)
            }

            Message::VersionsFetched(releases) => {
                self.available_versions = releases;
                self.selected_version_indices.clear();
                // Pre-select the latest version
                if !self.available_versions.is_empty() {
                    self.selected_version_indices.push(0);
                }

                // Check if any selected tool has architecture variants
                self.has_variant_tools = self.selected_tool_indices.iter().any(|&idx| {
                    self.available_tools
                        .get(idx)
                        .is_some_and(|t| t.has_multiple_asset_variations)
                });

                if self.has_variant_tools {
                    // Show architecture selection next
                    self.selection_step = SelectionStep::SelectingArchitecture;
                    self.selected_arch_variant = Some(2); // Default to v2
                } else {
                    // No variants, proceed to version selection
                    self.selection_step = SelectionStep::SelectingVersions;
                }
                Task::none()
            }

            Message::ToggleVersion(index) => {
                if let Some(pos) = self
                    .selected_version_indices
                    .iter()
                    .position(|&i| i == index)
                {
                    self.selected_version_indices.remove(pos);
                } else {
                    self.selected_version_indices.push(index);
                }
                Task::none()
            }

            Message::SelectArchitecture(variant_code) => {
                self.selected_arch_variant = Some(variant_code);
                Task::none()
            }

            Message::StartSelectedDownloads => {
                if self.selected_tool_indices.is_empty() || self.selected_version_indices.is_empty()
                {
                    self.global_status = "Please select tools and versions".to_string();
                    return Task::none();
                }

                // Build tools_and_versions for checking
                let mut tools_and_versions = Vec::new();
                for &tool_idx in &self.selected_tool_indices {
                    let tool = self.available_tools[tool_idx].clone();
                    let versions: Vec<Release> = self
                        .selected_version_indices
                        .iter()
                        .map(|&v_idx| self.available_versions[v_idx].clone())
                        .collect();
                    tools_and_versions.push((tool, versions));
                }

                let app_inst = self.app_installation.clone().unwrap();

                // Check which tools are already installed
                Task::perform(
                    Self::check_already_installed(app_inst, tools_and_versions),
                    Message::AlreadyInstalledChecked,
                )
            }

            Message::AlreadyInstalledChecked(already_installed) => {
                self.already_installed_tools = already_installed;
                self.force_reinstall_indices.clear();

                if self.already_installed_tools.is_empty() {
                    // Nothing to confirm, proceed directly to download
                    self.start_downloads(HashSet::new())
                } else {
                    // Show confirmation dialog
                    self.selection_step = SelectionStep::ConfirmReinstall;
                    self.global_status = format!(
                        "{} tool(s) already installed. Select which to reinstall.",
                        self.already_installed_tools.len()
                    );
                    Task::none()
                }
            }

            Message::QuickUpdateChecked(results) => {
                // Ignore if we're no longer in QuickUpdate mode (e.g., user cancelled)
                if self.app_mode != AppMode::QuickUpdate {
                    return Task::none();
                }
                
                let all_installed = results.iter().all(|(_, installed)| *installed);
                if all_installed && !results.is_empty() {
                    // All tools up to date - show force reinstall prompt
                    let tool_names: Vec<String> = results.into_iter().map(|(name, _)| name).collect();
                    self.quick_update_status = QuickUpdateStatus::AllUpToDate(tool_names);
                    self.global_status = "Tools are up to date.".to_string();
                    Task::none()
                } else {
                    // Some tools need updating - proceed with normal download
                    self.quick_update_status = QuickUpdateStatus::InProgress;
                    self.global_status = "Starting Quick Update...".to_string();
                    
                    // Pre-populate tools based on detected apps
                    self.tools.clear();
                    for app in &self.detected_apps {
                        let compat_tool = app.as_app().default_compatibility_tool();
                        self.tools
                            .push(ToolDownload::new(compat_tool.name, app.to_string()));
                    }

                    // Store the handle so we can abort the task later
                    let (task, handle) = download_task::run_quick_update(false);
                    self.download_handle = Some(handle);

                    task.map(Message::DownloadUpdate)
                }
            }

            Message::ForceReinstall => {
                self.quick_update_status = QuickUpdateStatus::InProgress;
                self.global_status = "Force reinstalling tools...".to_string();
                
                // Pre-populate tools based on detected apps
                self.tools.clear();
                for app in &self.detected_apps {
                    let compat_tool = app.as_app().default_compatibility_tool();
                    self.tools
                        .push(ToolDownload::new(compat_tool.name, app.to_string()));
                }

                // Store the handle so we can abort the task later
                let (task, handle) = download_task::run_quick_update(true);
                self.download_handle = Some(handle);

                task.map(Message::DownloadUpdate)
            }

            Message::ToggleReinstall(index) => {
                if let Some(pos) = self
                    .force_reinstall_indices
                    .iter()
                    .position(|&i| i == index)
                {
                    self.force_reinstall_indices.remove(pos);
                } else {
                    self.force_reinstall_indices.push(index);
                }
                Task::none()
            }

            Message::ConfirmReinstallSelection => {
                // Build the set of tool names that should be force reinstalled
                let force_reinstall_names: HashSet<String> = self
                    .force_reinstall_indices
                    .iter()
                    .filter_map(|&i| self.already_installed_tools.get(i))
                    .map(|t| t.name.clone())
                    .collect();
                self.start_downloads(force_reinstall_names)
            }

            Message::DownloadUpdate(update) => match update {
                DownloadUpdate::ToolProgress(progress) => {
                    if let Some(tool) = self.tools.iter_mut().find(|t| t.name == progress.tool_name)
                    {
                        tool.update_from_progress(&progress);
                    }
                    Task::none()
                }
                DownloadUpdate::GlobalProgress(progress) => {
                    self.global_phase = progress.phase;
                    self.global_status = progress.status_message;
                    self.global_progress = progress.percent;
                    Task::none()
                }
                DownloadUpdate::Finished(result) => {
                    // Clear the handle since the task is done
                    self.download_handle = None;

                    match result {
                        Ok(versions) => {
                            self.global_progress = 100.0;
                            self.global_phase = DownloadPhase::Complete;
                            self.global_status =
                                format!("✓ Success! Installed {} tools.", versions.len());
                            self.download_complete = Some(Ok(versions));
                            // Mark quick update as complete if we're in that mode
                            if self.app_mode == AppMode::QuickUpdate {
                                self.quick_update_status = QuickUpdateStatus::Complete;
                            }
                        }
                        Err(e) => {
                            let DownloadError::IoError(error_msg) = e;
                            self.global_phase = DownloadPhase::Error;
                            self.global_status = format!("✗ Error: {}", error_msg);
                            self.download_complete = Some(Err(error_msg));
                            // On error, stay in current state (InProgress) so user can retry
                        }
                    }
                    Task::none()
                }
            },

            Message::Cancel => {
                // Abort the download task
                if let Some(handle) = self.download_handle.take() {
                    handle.abort();
                }
                // Reset state to initial
                self.reset_to_initial();
                self.app_mode = AppMode::None;
                Task::none()
            }

            Message::BackToInitial => {
                self.reset_to_initial();
                self.app_mode = AppMode::None;
                Task::none()
            }

            Message::BackToToolSelection => {
                // Go back to tool selection step, clear version selection
                self.selection_step = SelectionStep::SelectingTools;
                self.selected_version_indices.clear();
                self.available_versions.clear();
                self.selected_tool = None;
                Task::none()
            }

            Message::SelectionError(e) => {
                self.global_status = format!("Error: {}", e);
                self.selection_step = SelectionStep::Initial;
                Task::none()
            }

            Message::TickSpinner => {
                // The Circular widget handles its own animation via RedrawRequested events
                Task::none()
            }

            Message::CloseRequested => {
                std::process::exit(0);
            }

            Message::SelectDownloadForCustom => {
                self.app_mode = AppMode::DownloadForCustom;
                self.mode = GuiMode::DownloadForCustom;
                self.selection_step = SelectionStep::Initial;
                self.custom_path_input = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.path_error = None;
                Task::none()
            }

            Message::CustomPathInput(path) => {
                self.custom_path_input = path;
                self.path_error = None;
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
                self.custom_path_input = path.to_string_lossy().to_string();
                self.path_error = None;
                Task::none()
            }

            Message::FolderPicked(None) => {
                // User cancelled picker
                Task::none()
            }

            Message::SelectManageInstallations => {
                self.app_mode = AppMode::ManageInstallations;
                self.mode = GuiMode::ManageInstallations;
                self.manage_status = "Scanning for installed versions...".to_string();
                self.manage_error = None;

                // Initialize app installation views with all selected (Detect All default)
                self.app_installations_views = libprotonup::apps::APP_INSTALLATIONS_VARIANTS
                    .iter()
                    .map(|app| AppInstallationView {
                        app: app.clone(),
                        selected: true,
                        versions: vec![],
                        loading: true,
                    })
                    .collect();

                Task::perform(
                    Self::scan_all_installed_versions(),
                    Message::VersionsScanned,
                )
            }

            Message::AppSelectionToggled(index) => {
                // If toggling an individual app, uncheck Detect All
                if let Some(view) = self.app_installations_views.get_mut(index) {
                    view.selected = !view.selected;
                }
                Task::none()
            }

            Message::VersionToggled(app_index, version_index) => {
                if let Some(view) = self.app_installations_views.get_mut(app_index)
                    && let Some(version) = view.versions.get_mut(version_index) {
                        version.selected_for_deletion = !version.selected_for_deletion;
                    }
                Task::none()
            }

            Message::DeleteSelectedVersions => {
                // Collect all selected versions with their paths
                let selected: Vec<(usize, usize, PathBuf)> = self
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
                    self.manage_status = "No versions selected for deletion".to_string();
                    return Task::none();
                }

                self.manage_status = format!("Deleting {} version(s)...", selected.len());

                Task::perform(Self::delete_versions(selected), Message::DeleteCompleted)
            }

            Message::DeleteCompleted(result) => {
                match result {
                    Ok(deleted) => {
                        self.manage_status = format!("✓ Deleted {} version(s)", deleted.len());
                        // Rescan to update the list
                        Task::perform(
                            Self::scan_all_installed_versions(),
                            Message::VersionsScanned,
                        )
                    }
                    Err(e) => {
                        self.manage_error = Some(e);
                        self.manage_status = "Error deleting versions".to_string();
                        Task::none()
                    }
                }
            }

            Message::VersionsScanned(versions) => {
                // Update the views with scanned versions
                for (i, view) in self.app_installations_views.iter_mut().enumerate() {
                    if let Some((_, vers)) = versions.get(i) {
                        view.versions = vers
                            .iter()
                            .map(|(parent_path, name)| InstalledVersion {
                                name: name.clone(),
                                path: parent_path.join(name), // Construct full path: parent_dir/version_name
                                selected_for_deletion: false,
                            })
                            .collect();
                        view.loading = false;
                    }
                }

                // Update status
                let total_versions: usize = self
                    .app_installations_views
                    .iter()
                    .map(|v| v.versions.len())
                    .sum();
                self.manage_status = format!(
                    "Found {} version(s) across {} app(s)",
                    total_versions,
                    self.app_installations_views.len()
                );
                Task::none()
            }
        }
    }

    async fn detect_app_and_fetch_tools(
        app: App,
    ) -> Result<(AppInstallations, Vec<CompatTool>), String> {
        // Detect installation
        let installations = app.detect_installation_method().await;
        if installations.is_empty() {
            return Err(format!("{} installation not found", app));
        }

        // Use first detected installation (could prompt user if multiple)
        let app_inst = installations[0].clone();

        // Get compatible tools
        let tools = CompatTool::sources_for_app(&app);
        if tools.is_empty() {
            return Err("No compatible tools found".to_string());
        }

        Ok((app_inst, tools))
    }

    /// Scan all app installations for installed versions
    async fn scan_all_installed_versions() -> Vec<(AppInstallations, Vec<(PathBuf, String)>)> {
        let mut results = vec![];
        for app in libprotonup::apps::APP_INSTALLATIONS_VARIANTS.iter() {
            let versions = app.list_installed_versions().await.unwrap_or_default();
            // Convert Folder to (PathBuf, String)
            let version_tuples: Vec<(PathBuf, String)> = versions
                .into_iter()
                .map(|f| (f.0 .0.clone(), f.0 .1.clone()))
                .collect();
            results.push((app.clone(), version_tuples));
        }
        results
    }

    /// Delete selected versions
    async fn delete_versions(
        selected: Vec<(usize, usize, PathBuf)>,
    ) -> Result<Vec<String>, String> {
        let mut deleted = vec![];
        for (_app_idx, _ver_idx, path) in selected {
            // Expand tilde in path if present
            let expanded_path =
                libprotonup::utils::expand_tilde(&path).unwrap_or_else(|| path.clone());

            if let Err(e) = tokio::fs::remove_dir_all(&expanded_path).await {
                eprintln!("Error deleting {}: {}", expanded_path.display(), e);
            } else {
                if let Some(name) = expanded_path.file_name() {
                    deleted.push(name.to_string_lossy().to_string());
                }
            }
        }
        Ok(deleted)
    }

    /// Check which tools are already installed
    async fn check_already_installed(
        app_installation: AppInstallations,
        tools_and_versions: Vec<(CompatTool, Vec<Release>)>,
    ) -> Vec<ToolDownload> {
        let mut already_installed = Vec::new();

        for (tool, versions) in &tools_and_versions {
            for version in versions {
                let install_name = tool.installation_name(&version.tag_name);
                let mut install_path =
                    PathBuf::from(app_installation.default_install_dir().as_str());
                install_path.push(&install_name);

                if files::check_if_exists(&install_path).await {
                    already_installed.push(ToolDownload::new(
                        format!("{} {}", tool.name, version.tag_name),
                        app_installation.to_string(),
                    ));
                }
            }
        }

        already_installed
    }

    /// Check if quick update tools are already installed
    async fn check_quick_update_installed(
        detected_apps: Vec<AppInstallations>,
    ) -> Vec<(String, bool)> {
        use libprotonup::downloads;
        use tokio::time::{timeout, Duration};

        let mut results = Vec::new();
        
        for app_inst in &detected_apps {
            let compat_tool = app_inst.as_app().default_compatibility_tool();
            let tool_name = compat_tool.name.clone();
            
            // Fetch latest release
            let release_result = timeout(
                Duration::from_secs(10),
                downloads::list_releases(&compat_tool)
            ).await;
            
            match release_result {
                Ok(Ok(mut release_list)) => {
                    if release_list.is_empty() {
                        results.push((tool_name, false)); // No releases available
                        continue;
                    }
                    
                    let latest_release = release_list.remove(0);
                    let install_name = compat_tool.installation_name(&latest_release.tag_name);
                    let mut install_path = PathBuf::from(app_inst.default_install_dir().as_str());
                    install_path.push(&install_name);
                    
                    // Check if already installed
                    let is_installed = files::check_if_exists(&install_path).await;
                    results.push((tool_name, is_installed));
                }
                Ok(Err(e)) => {
                    // Network error or API failure
                    eprintln!("Failed to fetch releases for {}: {}", tool_name, e);
                    results.push((tool_name, false));
                }
                Err(_) => {
                    // Timeout
                    eprintln!("Timeout fetching releases for {}", tool_name);
                    results.push((tool_name, false));
                }
            }
        }
        
        results
    }

    /// Start the actual downloads
    fn start_downloads(&mut self, force_reinstall_names: HashSet<String>) -> Task<Message> {
        self.selection_step = SelectionStep::Downloading;
        self.download_started = true;
        self.global_progress = 0.0;
        self.download_complete = None;

        // Prepare tools and versions for download
        let mut tools_and_versions = Vec::new();

        // Create ToolDownload entries for each tool/version combination
        self.tools.clear();
        for &tool_idx in &self.selected_tool_indices {
            let tool = self.available_tools[tool_idx].clone();
            let versions: Vec<Release> = self
                .selected_version_indices
                .iter()
                .map(|&v_idx| self.available_versions[v_idx].clone())
                .collect();

            // Create a ToolDownload entry for each version
            for version in &versions {
                self.tools.push(ToolDownload::new(
                    format!("{} {}", tool.name, version.tag_name),
                    self.app_installation
                        .as_ref()
                        .map(|a| a.to_string())
                        .unwrap_or_default(),
                ));
            }

            tools_and_versions.push((tool, versions));
        }

        let app_inst = self.app_installation.clone().unwrap();

        // Store the handle so we can abort the task later
        let (task, handle) = download_task::download_selected_tools(
            app_inst,
            tools_and_versions,
            force_reinstall_names,
            self.selected_arch_variant,
        );
        self.download_handle = Some(handle);

        task.map(Message::DownloadUpdate)
    }

    fn reset_to_initial(&mut self) {
        self.mode = GuiMode::Initial;
        self.selection_step = SelectionStep::Initial;
        self.available_tools.clear();
        self.selected_tool_indices.clear();
        self.selected_tool = None;
        self.available_versions.clear();
        self.selected_version_indices.clear();
        self.app_installation = None;
        self.download_started = false;
        self.tools.clear();
        self.global_phase = DownloadPhase::DetectingApps;
        self.global_status = String::new();
        self.global_progress = 0.0;
        self.download_complete = None;
        self.download_handle = None;

        // Clear manage installations state
        self.app_installations_views.clear();
        self.manage_status = String::new();
        self.manage_error = None;

        // Reset quick update state
        self.quick_update_status = QuickUpdateStatus::Idle;
    }

    fn view(&self) -> Element<'_, Message> {
        // Header
        let header = container(
            Row::new()
                .push(text("Protonup-rs").size(20))
                .push(space::horizontal())
                .push(text(&self.global_status).size(12))
                .padding(10)
                .align_y(Center),
        )
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style::default()
                .border(iced::border::color(palette.background.strong.color).width(1))
        });

        // Sidebar
        let sidebar = self.view_sidebar();

        // Main content
        let main_content = self.view_main_content();

        // Full layout
        Column::new()
            .push(header)
            .push(Row::new().push(sidebar).push(main_content))
            .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let mut column = Column::new().spacing(10).padding(10).width(220);

        // Logo at top
        let logo_handle = &self.logo_handle;
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

        // Check if currently downloading (not yet complete)
        let is_downloading = self.download_started
            && self.selection_step == SelectionStep::Downloading
            && self.download_complete.is_none();

        // Check if download is complete (success or error)
        let is_complete = self.download_complete.is_some();

        // Show loading spinner when downloading
        if is_downloading {
            column = column.push(
                Container::new(Circular::new().size(40.0).bar_height(4.0))
                    .center_x(Length::Fill)
                    .padding(10),
            );

            column = column.push(
                Container::new(text("Download in progress...").size(12)).center_x(Length::Fill),
            );
        }

        // Show completion status when done
        if is_complete {
            column = column.push(
                Container::new(text("Completed ✅").size(14))
                    .center_x(Length::Fill)
                    .padding(10),
            );
        }

        // Action buttons (disabled when downloading or when already selected)
        let quick_update_disabled = is_downloading || self.app_mode == AppMode::QuickUpdate;
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

        let steam_disabled = is_downloading || self.app_mode == AppMode::DownloadForSteam;
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

        let lutris_disabled = is_downloading || self.app_mode == AppMode::DownloadForLutris;
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

        let custom_disabled = is_downloading || self.app_mode == AppMode::DownloadForCustom;
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

        // Manage Existing Installations button
        let manage_disabled = is_downloading || self.app_mode == AppMode::ManageInstallations;
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

        // Cancel button (only when downloading)
        if is_downloading {
            column = column.push(
                button(text("Cancel").size(14))
                    .on_press(Message::Cancel)
                    .padding(10)
                    .width(Length::Fill)
                    .style(warning_button_style()),
            );
        }

        // Spacer to push close button to bottom
        column = column.push(space::vertical());

        // Close button at bottom
        column = column.push(
            button(text("Close").size(14))
                .on_press(Message::CloseRequested)
                .padding(10)
                .width(Length::Fill)
                .style(warning_button_style()),
        );

        container(column).style(container::rounded_box).into()
    }

    fn view_main_content(&self) -> Element<'_, Message> {
        let content: Element<Message> = {
            // If no action selected, show placeholder
            if self.app_mode == AppMode::None {
                container(center(text("⬅️ Choose your option").size(18)))
                    .width(Fill)
                    .height(Fill)
                    .into()
            }
            // Show download progress when downloading (but not for quick update checking/up-to-date states)
            else if self.download_started 
                && self.selection_step == SelectionStep::Downloading 
                && !matches!(self.quick_update_status, QuickUpdateStatus::Checking | QuickUpdateStatus::AllUpToDate(_)) {
                Column::new()
                    .spacing(10)
                    .push(self.view_download_progress())
                    .into()
            }
            // Otherwise show existing selection windows
            else {
                match &self.mode {
                    GuiMode::QuickUpdate => self.view_quick_update(),
                    GuiMode::DownloadForSteam
                    | GuiMode::DownloadForLutris
                    | GuiMode::DownloadForCustom => self.view_selection_flow(),
                    GuiMode::ManageInstallations => self.view_manage_installations(),
                    _ => container(center(text("⬅️ Choose your option").size(18)))
                        .width(Fill)
                        .height(Fill)
                        .into(),
                }
            }
        };

        // Wrap in container with padding
        container(content)
            .padding(20)
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_quick_update(&self) -> Element<'_, Message> {
        match &self.quick_update_status {
            QuickUpdateStatus::Checking => {
                Column::new()
                    .spacing(20)
                    .push(text("Checking for updates...").size(16))
                    .push(
                        Container::new(Circular::new().size(40.0).bar_height(4.0))
                            .center_x(Length::Fill)
                            .padding(10),
                    )
                    .into()
            }
            QuickUpdateStatus::AllUpToDate(tool_names) => {
                let mut column = Column::new().spacing(15);
                
                column = column.push(text("✓ Tools are up to date.").size(16).color([0.3, 1.0, 0.3]));
                
                column = column.push(text("The following tools are already installed:").size(14));
                
                for tool_name in tool_names {
                    column = column.push(
                        Row::new()
                            .spacing(10)
                            .push(text("•").size(14))
                            .push(text(tool_name).size(14))
                    );
                }
                
                column = column.push(space::vertical().height(Length::Fixed(20.0)));
                
                column = column.push(
                    button(text("Force Reinstallation").size(14))
                        .on_press(Message::ForceReinstall)
                        .padding(10)
                        .style(warning_button_style())
                );
                
                column = column.push(
                    button(text("Back to Main Menu").size(14))
                        .on_press(Message::BackToInitial)
                        .padding(10)
                );
                
                column.into()
            }
            QuickUpdateStatus::InProgress => {
                // Should not reach here - InProgress should show download progress via view_download_progress
                Column::new()
                    .spacing(10)
                    .push(text("Quick Update in progress...").size(14))
                    .into()
            }
            QuickUpdateStatus::Complete => {
                // Should not reach here - Complete should show completion via view_download_progress
                Column::new()
                    .spacing(10)
                    .push(text("Quick Update complete.").size(14))
                    .into()
            }
            QuickUpdateStatus::Idle => {
                // Should not reach here - Idle means not in QuickUpdate mode
                Column::new()
                    .spacing(10)
                    .push(text("Quick Update ready.").size(14))
                    .into()
            }
        }
    }

    fn view_selection_flow(&self) -> Element<'_, Message> {
        // Custom location needs path selection first
        if self.mode == GuiMode::DownloadForCustom && self.selection_step == SelectionStep::Initial
        {
            return self.view_custom_location_selection();
        }

        match &self.selection_step {
            SelectionStep::Initial => text("Initializing...").size(14).into(),
            SelectionStep::SelectingTools => self.view_tool_selection(),
            SelectionStep::SelectingVersions => self.view_version_selection(),
            SelectionStep::SelectingArchitecture => self.view_architecture_selection(),
            SelectionStep::ConfirmReinstall => self.view_confirm_reinstall(),
            SelectionStep::Downloading => {
                // Spinner is shown in sidebar, progress bars in main content
                text("Preparing downloads...").size(14).into()
            }
        }
    }

    fn view_tool_selection(&self) -> Element<'_, Message> {
        let app_name = match self.mode {
            GuiMode::DownloadForSteam => "Steam",
            GuiMode::DownloadForLutris => "Lutris",
            _ => "App",
        };

        let mut column = Column::new().spacing(10);
        column = column.push(text(format!("Select tool for {}:", app_name)).size(16));

        if self.available_tools.is_empty() {
            column = column.push(text("Loading tools...").size(14));
        } else {
            for (index, tool) in self.available_tools.iter().enumerate() {
                column = column.push(
                    Row::new()
                        .spacing(10)
                        .align_y(Center)
                        .push(radio(
                            "",
                            index,
                            self.selected_tool_indices.first().copied(),
                            Message::ToolSelected,
                        ))
                        .push(text(&tool.name).size(14)),
                );
            }
        }

        column = column.push(
            button(text("Continue").size(14))
                .on_press(Message::ToolSelectionConfirmed)
                .padding(10),
        );

        column = column.push(
            button(text("Back").size(14))
                .on_press(Message::BackToInitial)
                .padding(10),
        );

        scrollable(column).into()
    }

    fn view_version_selection(&self) -> Element<'_, Message> {
        let tool_name = self
            .selected_tool
            .as_ref()
            .map(|t| t.name.as_str())
            .unwrap_or("Tool");

        let mut column = Column::new().spacing(10);
        column = column.push(text(format!("Select versions for {}:", tool_name)).size(16));

        if self.available_versions.is_empty() {
            column = column.push(text("Loading versions...").size(14));
        } else {
            for (index, release) in self.available_versions.iter().enumerate() {
                let is_selected = self.selected_version_indices.contains(&index);
                column = column.push(
                    Row::new()
                        .spacing(10)
                        .push(
                            checkbox(is_selected).on_toggle(move |_| Message::ToggleVersion(index)),
                        )
                        .push(text(&release.tag_name).size(14)),
                );
            }
        }

        column = column.push(
            button(text("Start Download").size(14))
                .on_press(Message::StartSelectedDownloads)
                .padding(10),
        );

        column = column.push(
            button(text("Back").size(14))
                .on_press(Message::BackToToolSelection)
                .padding(10),
        );

        scrollable(column).into()
    }

    fn view_architecture_selection(&self) -> Element<'_, Message> {
        let mut column = Column::new().spacing(10);

        column = column.push(text("Select CPU Architecture Variant:").size(16));

        column = column.push(
            text("Some tools offer optimized builds for different CPU architectures.").size(12),
        );

        // Architecture variants: (code, name, description)
        let variants = [
            (1, "x86_64", "Universal - all x86-64 CPUs"),
            (2, "x86_64_v2", "Recommended - optimized for SSE3"),
            (3, "x86_64_v3", "Modern CPUs - optimized for AVX2"),
            (4, "x86_64_v4", "Experimental - optimized for AVX-512"),
        ];

        for (code, name, desc) in variants {
            let is_selected = self.selected_arch_variant == Some(code);
            column = column.push(
                Row::new()
                    .spacing(10)
                    .align_y(Center)
                    .push(
                        checkbox(is_selected).on_toggle(move |_| Message::SelectArchitecture(code)),
                    )
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

    fn view_custom_location_selection(&self) -> Element<'_, Message> {
        let mut column = Column::new().spacing(10);

        column = column.push(text("Select Installation Directory:").size(16));

        column = column.push(
            text("Enter a path or use the folder picker to select where compatibility tools will be installed.").size(12)
        );

        // Text input with current path
        column = column.push(
            text_input("Enter path...", &self.custom_path_input)
                .on_input(Message::CustomPathInput)
                .padding(10),
        );

        // Folder picker button
        column = column.push(
            button(text("📁 Browse...").size(14))
                .on_press(Message::OpenFolderPicker)
                .padding(10),
        );

        // Show error if any
        if let Some(ref error) = self.path_error {
            column = column.push(text(error).size(12).color([1.0, 0.3, 0.3]));
        }

        // Continue button
        column = column.push(
            button(text("Continue").size(14))
                .on_press(Message::ToolSelectionConfirmed)
                .padding(10),
        );

        // Back button
        column = column.push(
            button(text("Back").size(14))
                .on_press(Message::BackToInitial)
                .padding(10),
        );

        scrollable(column).into()
    }

    fn view_manage_installations(&self) -> Element<'_, Message> {
        let mut main_column = Column::new().spacing(20);

        // Status message
        main_column = main_column.push(text(&self.manage_status).size(14));

        if let Some(ref error) = self.manage_error {
            main_column = main_column.push(text(error).size(12).color([1.0, 0.3, 0.3]));
        }

        // App installations grid (side by side)
        let mut row = Row::new().spacing(20);

        for (app_idx, view) in self.app_installations_views.iter().enumerate() {
            let mut col = Column::new().spacing(10).width(Length::Fill);

            // App title with checkbox
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

        // Delete button
        let has_selections = self
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

        // Back button
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

    fn view_confirm_reinstall(&self) -> Element<'_, Message> {
        let mut column = Column::new().spacing(10);

        column = column.push(text("The following tools are already installed:").size(16));

        column = column.push(text("Select which ones you want to reinstall:").size(14));

        if self.already_installed_tools.is_empty() {
            column = column.push(text("No tools to reinstall.").size(14));
        } else {
            for (index, tool) in self.already_installed_tools.iter().enumerate() {
                let is_selected = self.force_reinstall_indices.contains(&index);
                column = column.push(
                    Row::new()
                        .spacing(10)
                        .push(
                            checkbox(is_selected)
                                .on_toggle(move |_| Message::ToggleReinstall(index)),
                        )
                        .push(text(&tool.name).size(14)),
                );
            }
        }

        column = column.push(
            button(text("Continue").size(14))
                .on_press(Message::ConfirmReinstallSelection)
                .padding(10),
        );

        column = column.push(
            button(text("Back").size(14))
                .on_press(Message::BackToInitial)
                .padding(10),
        );

        scrollable(column).into()
    }

    fn view_download_progress(&self) -> Element<'_, Message> {
        let mut column = Column::new().spacing(10);

        for tool in &self.tools {
            let status_color = match &tool.status {
                ToolStatus::_Complete => [0.3, 1.0, 0.3],
                ToolStatus::Error(_) => [1.0, 0.3, 0.3],
                _ => [1.0, 1.0, 1.0],
            };

            column = column.push(
                Column::new()
                    .spacing(5)
                    .push(text(format!("{} for {}", tool.name, tool.app_target)).size(12))
                    .push(progress_bar(0.0..=100.0, tool.progress))
                    .push(text(tool.status_text()).size(10).color(status_color)),
            );
        }

        // Completion actions
        if let Some(ref result) = self.download_complete {
            match result {
                Ok(versions) => {
                    column = column
                        .push(text("✓ All tools installed successfully!").color([0.3, 1.0, 0.3]));
                    for version in versions {
                        column = column.push(text(format!("  • {}", version)).size(12));
                    }
                    column = column.push(
                        button(text("Back to Main Menu").size(14))
                            .on_press(Message::BackToInitial)
                            .padding(5),
                    );
                }
                Err(e) => {
                    column = column.push(text(format!("✗ Failed: {}", e)).color([1.0, 0.3, 0.3]));
                    column = column.push(
                        button(text("Try Again").size(14))
                            .on_press(Message::StartSelectedDownloads)
                            .padding(5),
                    );
                }
            }
        }

        column.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![];

        // App scanning subscription
        if !self.scan_complete {
            subscriptions.push(time::every(Duration::from_millis(100)).map(|_| Message::ScanApps));
        }

        // Window frames subscription for spinner animation when downloading
        if self.download_started && self.selection_step == SelectionStep::Downloading {
            subscriptions.push(window::frames().map(|_| Message::TickSpinner));
        }

        Subscription::batch(subscriptions)
    }
}

pub fn main() -> iced::Result {
    iced::application(ProtonupGui::default, ProtonupGui::update, ProtonupGui::view)
        .subscription(ProtonupGui::subscription)
        .run()
}
