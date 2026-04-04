use iced::widget::{button, center, checkbox, progress_bar, text, Column, Container, Row, scrollable};
use iced::task;
use iced::{Center, Element, Task, Subscription, Length};
use iced::time;

use libprotonup::apps::{list_installed_apps, App, AppInstallations};
use libprotonup::sources::CompatTool;
use libprotonup::downloads::Release;
use libprotonup::files;

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

// Spinner animation constants
const SPINNER_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_INTERVAL: Duration = Duration::from_millis(80);

mod download;
mod download_task;
use download_task::{DownloadUpdate, ToolProgress};
use download::DownloadPhase;

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
    ToolsFetched(Vec<CompatTool>),  // Unused, kept for compatibility
    AppInstallationDetected(AppInstallations),
    ToggleTool(usize),
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

    // Navigation
    BackToInitial,
    Restart,

    // Errors
    SelectionError(String),

    // Spinner animation
    TickSpinner,

    // Cancel download
    Cancel,
}

/// GUI mode - what the user is doing
#[derive(Debug, Clone, PartialEq, Default)]
enum GuiMode {
    #[default]
    Initial,
    QuickUpdate,
    DownloadForSteam,
    DownloadForLutris,
}

/// Current step in the selection flow
#[derive(Debug, Clone, PartialEq, Default)]
enum SelectionStep {
    #[default]
    Initial,
    SelectingTools,
    SelectingVersions,
    SelectingArchitecture,  // NEW: Show architecture variant selection
    ConfirmReinstall,
    Downloading,
    Complete,
}

/// Tracks the state of a single tool download
#[derive(Debug, Clone)]
struct ToolDownload {
    name: String,
    app_target: String,
    version: Option<String>,
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
    Complete,
    Error(String),
}

impl ToolDownload {
    fn new(name: String, app_target: String) -> Self {
        Self {
            name,
            app_target,
            version: None,
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
                self.status = ToolStatus::Complete;
            }
            DownloadPhase::Error => {
                self.status = ToolStatus::Error(progress.status_message.clone());
            }
        }
    }

    fn status_text(&self) -> String {
        match &self.status {
            ToolStatus::Pending => format!("{} - Waiting...", self.name),
            ToolStatus::Downloading => format!("{} - Downloading... {:.1}%", self.name, self.progress),
            ToolStatus::Validating => format!("{} - Validating... {:.1}%", self.name, self.progress),
            ToolStatus::Unpacking => format!("{} - Installing... {:.1}%", self.name, self.progress),
            ToolStatus::Complete => format!("{} - ✓ Installed", self.name),
            ToolStatus::Error(msg) => format!("{} - ✗ Error: {}", self.name, msg),
        }
    }
}

#[derive(Debug, Default)]
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
    selected_arch_variant: Option<u8>,  // 1=x86_64, 2=v2, 3=v3, 4=v4
    has_variant_tools: bool,            // True if any selected tool has variants

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

    // Spinner animation state
    spinner_frame: usize,

    // Cancel handle for aborting downloads
    download_handle: Option<task::Handle>,
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

            Message::ToolsFetched(_tools) => {
                // This message is no longer used - tools are fetched via AppInstallationDetected
                Task::none()
            }

            Message::SelectQuickUpdate => {
                self.mode = GuiMode::QuickUpdate;
                self.selection_step = SelectionStep::Downloading;
                self.download_started = true;
                self.global_progress = 0.0;
                self.download_complete = None;
                self.global_status = "Starting Quick Update...".to_string();

                // Pre-populate tools based on detected apps
                self.tools.clear();
                for app in &self.detected_apps {
                    let compat_tool = app.as_app().default_compatibility_tool();
                    self.tools.push(ToolDownload::new(
                        compat_tool.name,
                        app.to_string(),
                    ));
                }

                // Store the handle so we can abort the task later
                let (task, handle) = download_task::run_quick_update(false);
                self.download_handle = Some(handle);

                task.map(Message::DownloadUpdate)
            }

            Message::SelectDownloadForSteam => {
                self.mode = GuiMode::DownloadForSteam;
                self.selection_step = SelectionStep::SelectingTools;
                self.global_status = "Detecting Steam installation...".to_string();
                Task::perform(
                    Self::detect_app_and_fetch_tools(App::Steam),
                    |result| match result {
                        Ok((app_inst, tools)) => Message::AppInstallationDetected(app_inst),
                        Err(e) => Message::SelectionError(e),
                    }
                )
            }

            Message::SelectDownloadForLutris => {
                self.mode = GuiMode::DownloadForLutris;
                self.selection_step = SelectionStep::SelectingTools;
                self.global_status = "Detecting Lutris installation...".to_string();
                Task::perform(
                    Self::detect_app_and_fetch_tools(App::Lutris),
                    |result| match result {
                        Ok((app_inst, tools)) => Message::AppInstallationDetected(app_inst),
                        Err(e) => Message::SelectionError(e),
                    }
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

            Message::ToggleTool(index) => {
                if let Some(pos) = self.selected_tool_indices.iter().position(|&i| i == index) {
                    self.selected_tool_indices.remove(pos);
                } else {
                    self.selected_tool_indices.push(index);
                }
                Task::none()
            }

            Message::ToolSelectionConfirmed => {
                if self.selected_tool_indices.is_empty() {
                    self.global_status = "Please select at least one tool".to_string();
                    return Task::none();
                }
                
                // Get the first selected tool for version selection
                let tool = self.available_tools[self.selected_tool_indices[0]].clone();
                self.selected_tool = Some(tool.clone());
                self.selection_step = SelectionStep::SelectingVersions;
                self.global_status = format!("Fetching releases for {}...", tool.name);
                
                Task::perform(
                    download::fetch_releases(tool),
                    Message::VersionsFetched
                )
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
                    self.available_tools.get(idx).map_or(false, |t| t.has_multiple_asset_variations)
                });

                if self.has_variant_tools {
                    // Show architecture selection next
                    self.selection_step = SelectionStep::SelectingArchitecture;
                    self.selected_arch_variant = Some(2);  // Default to v2
                } else {
                    // No variants, proceed to version selection
                    self.selection_step = SelectionStep::SelectingVersions;
                }
                Task::none()
            }

            Message::ToggleVersion(index) => {
                if let Some(pos) = self.selected_version_indices.iter().position(|&i| i == index) {
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
                if self.selected_tool_indices.is_empty() || self.selected_version_indices.is_empty() {
                    self.global_status = "Please select tools and versions".to_string();
                    return Task::none();
                }

                // Build tools_and_versions for checking
                let mut tools_and_versions = Vec::new();
                for &tool_idx in &self.selected_tool_indices {
                    let tool = self.available_tools[tool_idx].clone();
                    let versions: Vec<Release> = self.selected_version_indices
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

            Message::ToggleReinstall(index) => {
                if let Some(pos) = self.force_reinstall_indices.iter().position(|&i| i == index) {
                    self.force_reinstall_indices.remove(pos);
                } else {
                    self.force_reinstall_indices.push(index);
                }
                Task::none()
            }

            Message::ConfirmReinstallSelection => {
                // Build the set of tool names that should be force reinstalled
                let force_reinstall_names: HashSet<String> = self.force_reinstall_indices
                    .iter()
                    .filter_map(|&i| self.already_installed_tools.get(i))
                    .map(|t| t.name.clone())
                    .collect();
                self.start_downloads(force_reinstall_names)
            }

            Message::DownloadUpdate(update) => match update {
                DownloadUpdate::ToolProgress(progress) => {
                    if let Some(tool) = self.tools.iter_mut().find(|t| t.name == progress.tool_name) {
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
                            self.global_status = format!(
                                "✓ Success! Installed {} tools.",
                                versions.len()
                            );
                            self.download_complete = Some(Ok(versions));
                        }
                        Err(e) => {
                            self.global_phase = DownloadPhase::Error;
                            self.global_status = format!("✗ Error: {:?}", e);
                            self.download_complete = Some(Err(format!("{:?}", e)));
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
                Task::none()
            }

            Message::BackToInitial => {
                self.reset_to_initial();
                Task::none()
            }

            Message::Restart => {
                self.reset_to_initial();
                Task::perform(list_installed_apps(), Message::AppsScanned)
            }

            Message::SelectionError(e) => {
                self.global_status = format!("Error: {}", e);
                self.selection_step = SelectionStep::Initial;
                Task::none()
            }

            Message::TickSpinner => {
                self.spinner_frame = (self.spinner_frame + 1) % SPINNER_CHARS.len();
                Task::none()
            }
        }
    }

    async fn detect_app_and_fetch_tools(app: App) -> Result<(AppInstallations, Vec<CompatTool>), String> {
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

    /// Check which tools are already installed
    async fn check_already_installed(
        app_installation: AppInstallations,
        tools_and_versions: Vec<(CompatTool, Vec<Release>)>,
    ) -> Vec<ToolDownload> {
        let mut already_installed = Vec::new();

        for (tool, versions) in &tools_and_versions {
            for version in versions {
                let install_name = tool.installation_name(&version.tag_name);
                let mut install_path = PathBuf::from(app_installation.default_install_dir().as_str());
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
            let versions: Vec<Release> = self.selected_version_indices
                .iter()
                .map(|&v_idx| self.available_versions[v_idx].clone())
                .collect();

            // Create a ToolDownload entry for each version
            for version in &versions {
                self.tools.push(ToolDownload::new(
                    format!("{} {}", tool.name, version.tag_name),
                    self.app_installation.as_ref().map(|a| a.to_string()).unwrap_or_default(),
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
    }

    fn view(&self) -> Element<Message> {
        let mut content = Column::new().spacing(20).padding(20);

        // Title
        content = content.push(text("Protonup-rs GUI").size(24));

        // App detection status
        if self.scan_complete {
            let apps_text = if self.detected_apps.is_empty() {
                text("No compatible apps detected").color([1.0, 0.3, 0.3])
            } else {
                text(&self.global_status)
            };
            content = content.push(apps_text.size(14));
        } else {
            content = content.push(text(&self.global_status).size(14));
        }

        // Main content based on mode and step
        match &self.mode {
            GuiMode::Initial => {
                content = content.push(self.view_initial_buttons());
            }
            GuiMode::QuickUpdate => {
                content = content.push(self.view_quick_update());
            }
            GuiMode::DownloadForSteam | GuiMode::DownloadForLutris => {
                content = content.push(self.view_selection_flow());
            }
        }

        // Download progress section (always shown when downloading)
        if self.download_started {
            content = content.push(text("Download Progress:").size(16));
            content = content.push(self.view_download_progress());
        }

        // Cancel button at bottom (always visible when downloading)
        if self.download_started && self.selection_step == SelectionStep::Downloading {
            content = content.push(
                Container::new(
                    button(text("Cancel").size(14))
                        .on_press(Message::Cancel)
                        .padding(10),
                )
                .center_x(Length::Fill)
                .width(Length::Fill),
            );
        }

        Container::new(center(content)).into()
    }

    fn view_initial_buttons(&self) -> Element<Message> {
        let mut row = Row::new().spacing(10);
        
        if !self.scan_complete {
            row = row.push(text("Scanning...").size(14));
        } else if self.detected_apps.is_empty() {
            row = row.push(text("No apps detected").color([1.0, 0.3, 0.3]).size(14));
        } else {
            row = row.push(
                button(text("Quick Update").size(14))
                    .on_press(Message::SelectQuickUpdate)
                    .padding(10),
            );
            row = row.push(
                button(text("Download for Steam").size(14))
                    .on_press(Message::SelectDownloadForSteam)
                    .padding(10),
            );
            row = row.push(
                button(text("Download for Lutris").size(14))
                    .on_press(Message::SelectDownloadForLutris)
                    .padding(10),
            );
        }
        
        row.into()
    }

    fn view_quick_update(&self) -> Element<Message> {
        Column::new()
            .spacing(10)
            .push(text("Quick Update in progress...").size(14))
            .into()
    }

    fn view_selection_flow(&self) -> Element<Message> {
        match &self.selection_step {
            SelectionStep::Initial => {
                text("Initializing...").size(14).into()
            }
            SelectionStep::SelectingTools => {
                self.view_tool_selection()
            }
            SelectionStep::SelectingVersions => {
                self.view_version_selection()
            }
            SelectionStep::SelectingArchitecture => {
                self.view_architecture_selection()
            }
            SelectionStep::ConfirmReinstall => {
                self.view_confirm_reinstall()
            }
            SelectionStep::Downloading => {
                let spinner = SPINNER_CHARS[self.spinner_frame];
                Column::new()
                    .spacing(10)
                    .align_x(Center)
                    .push(text(spinner).size(48))
                    .push(text("Download in progress...").size(16))
                    .into()
            }
            SelectionStep::Complete => {
                Column::new()
                    .spacing(10)
                    .push(text("Download complete!").size(14))
                    .push(
                        button(text("Back to Main Menu").size(14))
                            .on_press(Message::BackToInitial)
                            .padding(10),
                    )
                    .into()
            }
        }
    }

    fn view_tool_selection(&self) -> Element<Message> {
        let app_name = match self.mode {
            GuiMode::DownloadForSteam => "Steam",
            GuiMode::DownloadForLutris => "Lutris",
            _ => "App",
        };

        let mut column = Column::new().spacing(10);
        column = column.push(text(format!("Select tools for {}:", app_name)).size(16));

        if self.available_tools.is_empty() {
            column = column.push(text("Loading tools...").size(14));
        } else {
            for (index, tool) in self.available_tools.iter().enumerate() {
                let is_selected = self.selected_tool_indices.contains(&index);
                column = column.push(
                    Row::new()
                        .spacing(10)
                        .push(checkbox(is_selected).on_toggle(move |_| Message::ToggleTool(index)))
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

    fn view_version_selection(&self) -> Element<Message> {
        let tool_name = self.selected_tool.as_ref().map(|t| t.name.as_str()).unwrap_or("Tool");

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
                        .push(checkbox(is_selected).on_toggle(move |_| Message::ToggleVersion(index)))
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
                .on_press(Message::ToolSelectionConfirmed)
                .padding(10),
        );

        scrollable(column).into()
    }

    fn view_architecture_selection(&self) -> Element<Message> {
        let mut column = Column::new().spacing(10);

        column = column.push(
            text("Select CPU Architecture Variant:").size(16)
        );

        column = column.push(
            text("Some tools offer optimized builds for different CPU architectures.")
                .size(12)
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
                    .push(checkbox(is_selected).on_toggle(move |_| Message::SelectArchitecture(code)))
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

    fn view_confirm_reinstall(&self) -> Element<Message> {
        let mut column = Column::new().spacing(10);

        column = column.push(
            text("The following tools are already installed:").size(16)
        );

        column = column.push(
            text("Select which ones you want to reinstall:").size(14)
        );

        if self.already_installed_tools.is_empty() {
            column = column.push(text("No tools to reinstall.").size(14));
        } else {
            for (index, tool) in self.already_installed_tools.iter().enumerate() {
                let is_selected = self.force_reinstall_indices.contains(&index);
                column = column.push(
                    Row::new()
                        .spacing(10)
                        .push(checkbox(is_selected).on_toggle(move |_| Message::ToggleReinstall(index)))
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

    fn view_download_progress(&self) -> Element<Message> {
        let mut column = Column::new().spacing(10);

        for tool in &self.tools {
            let status_color = match &tool.status {
                ToolStatus::Complete => [0.3, 1.0, 0.3],
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
                    column = column.push(
                        text("✓ All tools installed successfully!").color([0.3, 1.0, 0.3]),
                    );
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
                    column = column.push(
                        text(format!("✗ Failed: {}", e)).color([1.0, 0.3, 0.3]),
                    );
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
            subscriptions.push(
                time::every(Duration::from_millis(100))
                    .map(|_| Message::ScanApps)
            );
        }

        // Spinner animation subscription when downloading
        if self.download_started && self.selection_step == SelectionStep::Downloading {
            subscriptions.push(
                time::every(SPINNER_INTERVAL)
                    .map(|_| Message::TickSpinner)
            );
        }

        Subscription::batch(subscriptions)
    }
}

pub fn main() -> iced::Result {
    iced::application(ProtonupGui::default, ProtonupGui::update, ProtonupGui::view)
        .subscription(ProtonupGui::subscription)
        .run()
}
