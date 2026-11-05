use iced::task;
use iced::widget::button;
use iced::{Border, Color, Task};
use std::collections::HashSet;
use std::path::PathBuf;

use libprotonup::apps::{App, AppInstallations};
use libprotonup::architecture_variants::MicroArchVariants;
use libprotonup::files;
use libprotonup::sources::CompatTool;

use crate::download::DownloadPhase;
use crate::download_task;
use crate::message::{
    AppInstallationView, AppMode, GuiMode, InstalledTool, Message, QuickUpdateStatus,
    SelectionStep, ToolDownload,
};

use crate::LOGO_BYTES;

#[derive(Debug)]
pub(crate) struct ProtonupGui {
    pub detected_apps: Vec<AppInstallations>,
    pub scan_complete: bool,

    pub mode: GuiMode,
    pub selection_step: SelectionStep,

    pub available_tools: Vec<CompatTool>,
    pub selected_tool_indices: Vec<usize>,

    pub selected_tool: Option<CompatTool>,
    pub available_versions: Vec<libprotonup::downloads::Release>,
    pub selected_version_indices: Vec<usize>,

    pub selected_arch_variant: Option<MicroArchVariants>,
    pub has_variant_tools: bool,

    pub app_installation: Option<AppInstallations>,

    pub already_installed_tools: Vec<ToolDownload>,
    pub force_reinstall_indices: Vec<usize>,

    pub download_started: bool,
    pub tools: Vec<ToolDownload>,
    pub global_phase: DownloadPhase,
    pub global_status: String,
    pub global_progress: f32,
    pub download_complete:
        Option<Result<Vec<(libprotonup::downloads::Release, CompatTool)>, String>>,

    pub download_handle: Option<task::Handle>,

    pub app_mode: AppMode,

    pub custom_path_input: String,
    pub path_error: Option<String>,

    pub app_installations_views: Vec<AppInstallationView>,
    pub manage_status: String,
    pub manage_error: Option<String>,

    pub quick_update_status: QuickUpdateStatus,

    pub show_changelog: Option<(libprotonup::downloads::Release, CompatTool)>,

    pub show_all_versions: Option<InstalledTool>,

    pub logo_handle: iced::widget::image::Handle,

    /// Keyboard/controller focus state.
    pub focus: FocusState,

    /// Whether the keyboard shortcuts help panel is open.
    pub show_keyboard_shortcuts: bool,
    /// Whether the controller shortcuts help panel is open.
    pub show_controller_shortcuts: bool,
    /// Set when gamepad support could not be initialized (headless, no udev, …).
    pub gamepads_unavailable: bool,
}

/// Where keyboard/controller focus currently is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FocusState {
    /// Only draw highlights after the first navigation input.
    pub engaged: bool,
    pub pane: crate::message::Pane,
    pub row: usize,
    pub col: usize,
}

impl Default for FocusState {
    fn default() -> Self {
        Self {
            engaged: false,
            pane: crate::message::Pane::Sidebar,
            row: 0,
            col: 0,
        }
    }
}

impl FocusState {
    /// Turns on focus highlighting at the current cursor position.
    pub fn engage(&mut self) {
        self.engaged = true;
    }

    /// Re-anchors the focus cursor after the visible layout changed.
    ///
    /// Views call this whenever they advance to another step so the highlight
    /// cannot point at an option that no longer exists.
    pub fn focus_pane(&mut self, pane: crate::message::Pane) {
        self.engaged = true;
        self.pane = pane;
        self.row = 0;
        self.col = 0;
    }

    /// Re-anchors focus on the main pane when highlighting is already active.
    ///
    /// This keeps the user in the content pane across step changes without
    /// forcing highlights on for people who never used the keyboard/controller.
    pub fn reanchor_after_view_change(&mut self) {
        if self.engaged {
            self.focus_pane(crate::message::Pane::Main);
        }
    }
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
            global_phase: DownloadPhase::DetectingApps,
            global_status: String::new(),
            global_progress: 0.0,
            download_complete: None::<Result<Vec<_>, _>>,
            download_handle: None,
            app_mode: AppMode::default(),
            custom_path_input: String::new(),
            path_error: None,
            app_installations_views: Vec::new(),
            manage_status: String::new(),
            manage_error: None,
            quick_update_status: QuickUpdateStatus::default(),
            show_changelog: None,
            show_all_versions: None,
            logo_handle: iced::widget::image::Handle::from_bytes(LOGO_BYTES),
            focus: FocusState::default(),
            show_keyboard_shortcuts: false,
            show_controller_shortcuts: false,
            gamepads_unavailable: false,
        }
    }
}

pub(crate) fn warning_button_style()
-> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    |theme, status| {
        let palette = theme.extended_palette();
        let warning_color = Color::from_rgb(0.6, 0.5, 0.2);

        match status {
            button::Status::Hovered => button::Style {
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
            button::Status::Active => button::Style {
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
            button::Status::Pressed => button::Style {
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
            button::Status::Disabled => button::Style {
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
    pub fn reset_to_initial(&mut self) {
        self.mode = GuiMode::Initial;
        self.selection_step = SelectionStep::Initial;
        self.available_tools.clear();
        self.selected_tool_indices.clear();
        self.selected_tool = None;
        self.available_versions.clear();
        self.selected_version_indices.clear();
        self.show_changelog = None;
        self.app_installation = None;
        self.download_started = false;
        self.tools.clear();
        self.global_phase = DownloadPhase::DetectingApps;
        self.global_status = String::new();
        self.global_progress = 0.0;
        self.download_complete = None;
        self.download_handle = None;

        self.app_installations_views.clear();
        self.manage_status = String::new();
        self.manage_error = None;

        self.quick_update_status = QuickUpdateStatus::Idle;

        self.focus = FocusState::default();
    }

    pub fn start_downloads(&mut self, force_reinstall_names: HashSet<String>) -> Task<Message> {
        self.selection_step = SelectionStep::Downloading;
        self.download_started = true;
        self.global_progress = 0.0;
        self.download_complete = None;

        let mut tools_and_versions = Vec::new();

        self.tools.clear();
        for &tool_idx in &self.selected_tool_indices {
            let tool = self.available_tools[tool_idx].clone();
            let versions: Vec<libprotonup::downloads::Release> = self
                .selected_version_indices
                .iter()
                .map(|&v_idx| self.available_versions[v_idx].clone())
                .collect();

            for version in &versions {
                self.tools.push(ToolDownload::new(format!(
                    "{} {}",
                    tool.name, version.tag_name
                )));
            }

            tools_and_versions.push((tool, versions));
        }

        let app_inst = self.app_installation.clone().unwrap();

        let (task, handle) = download_task::download_selected_tools(
            app_inst,
            tools_and_versions,
            force_reinstall_names,
            self.selected_arch_variant,
        );
        self.download_handle = Some(handle);

        task.map(Message::DownloadUpdate)
    }

    pub async fn detect_app_and_fetch_tools(
        app: App,
    ) -> Result<(AppInstallations, Vec<CompatTool>), String> {
        let installations = app.detect_installation_method().await;
        if installations.is_empty() {
            return Err(format!("{} installation not found", app));
        }

        let app_inst = installations[0].clone();

        let tools = CompatTool::sources_for_app(&app);
        if tools.is_empty() {
            return Err("No compatible tools found".to_string());
        }

        Ok((app_inst, tools))
    }

    pub async fn scan_all_installed_versions() -> Vec<(AppInstallations, Vec<(PathBuf, String)>)> {
        let mut results = vec![];
        for app in libprotonup::apps::APP_INSTALLATIONS_VARIANTS.iter() {
            let versions = app.list_installed_versions().await.unwrap_or_default();
            let version_tuples: Vec<(PathBuf, String)> = versions
                .into_iter()
                .map(|f| (f.0.0.clone(), f.0.1.clone()))
                .collect();
            results.push((app.clone(), version_tuples));
        }
        results
    }

    pub async fn delete_versions(
        selected: Vec<(usize, usize, PathBuf)>,
    ) -> Result<Vec<String>, String> {
        let mut deleted = vec![];
        for (_app_idx, _ver_idx, path) in selected {
            let expanded_path =
                libprotonup::utils::expand_tilde(&path).unwrap_or_else(|| path.clone());

            if let Err(e) = tokio::fs::remove_dir_all(&expanded_path).await {
                eprintln!("Error deleting {}: {}", expanded_path.display(), e);
            } else if let Some(name) = expanded_path.file_name() {
                deleted.push(name.to_string_lossy().to_string());
            }
        }
        Ok(deleted)
    }

    pub async fn check_already_installed(
        app_installation: AppInstallations,
        tools_and_versions: Vec<(CompatTool, Vec<libprotonup::downloads::Release>)>,
    ) -> Vec<ToolDownload> {
        let mut already_installed = Vec::new();

        for (tool, versions) in &tools_and_versions {
            for version in versions {
                let install_name = tool.installation_name(&version.tag_name);
                let mut install_path =
                    PathBuf::from(app_installation.default_install_dir().as_str());
                install_path.push(&install_name);

                if files::check_if_exists(&install_path).await {
                    already_installed.push(ToolDownload::new(format!(
                        "{} {}",
                        tool.name, version.tag_name
                    )));
                }
            }
        }

        already_installed
    }

    pub(crate) async fn check_quick_update_installed(
        detected_apps: Vec<AppInstallations>,
    ) -> Vec<InstalledTool> {
        use crate::message::cmp_versions;

        let mut results = Vec::new();

        for app_inst in &detected_apps {
            let compat_tool = app_inst.as_app().default_compatibility_tool();
            let tool_name = compat_tool.name.clone();

            let installed_versions = match app_inst.list_installed_versions().await {
                Ok(versions) => versions
                    .into_iter()
                    .map(|folder| folder.0.1)
                    .collect::<Vec<String>>(),
                Err(_) => Vec::new(),
            };

            // Versions belonging to the app's default tool (folder naming convention).
            let tool_versions = installed_versions
                .iter()
                .filter(|name| name.starts_with("GE-Proton") || name.starts_with("Proton-"));

            // Latest installed version of the default tool (natural ordering).
            let latest = tool_versions.max_by(|a, b| cmp_versions(a, b)).cloned();

            // Full installed list for this app (all tools), newest first.
            let mut versions = installed_versions;
            versions.sort_by(|a, b| cmp_versions(b, a));

            results.push(InstalledTool {
                app: app_inst.clone(),
                tool: tool_name,
                latest,
                versions,
            });
        }

        results
    }

    pub fn subscription(&self) -> iced::Subscription<crate::message::Message> {
        let mut subscriptions = vec![];

        if !self.scan_complete {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_millis(100))
                    .map(|_| crate::message::Message::ScanApps),
            );
        }

        if self.download_started
            && self.selection_step == crate::message::SelectionStep::Downloading
            && !matches!(
                self.quick_update_status,
                crate::message::QuickUpdateStatus::Checking
                    | crate::message::QuickUpdateStatus::AllUpToDate(_)
            )
        {
            subscriptions
                .push(iced::window::frames().map(|_| crate::message::Message::TickSpinner));
        }

        // Keyboard navigation (only sees events no widget consumed).
        subscriptions.push(
            iced::keyboard::listen().filter_map(|event| crate::input::keyboard_to_message(&event)),
        );

        // Gamepad support runs in a background task so the UI thread never
        // blocks on `Gilrs` initialization or event polling.
        if !self.gamepads_unavailable {
            subscriptions.push(gamepad_subscription());
        }

        iced::Subscription::batch(subscriptions)
    }
}

/// Runs the gamepad backend in a background task, off the UI thread.
pub(crate) fn gamepad_subscription() -> iced::Subscription<Message> {
    iced::Subscription::run(gamepad_stream)
}

fn gamepad_stream() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::sink::SinkExt;

    iced::stream::channel(64, async |mut output| {
        let mut backend = match gilrs::Gilrs::new() {
            Ok(backend) => Some(backend),
            Err(_) => {
                let _ = output.send(Message::GamepadUnavailable).await;
                return;
            }
        };

        // TODO: not needed. All connected controllers should trigger the same action
        // Per-gamepad edge-triggering state for the left stick, so several
        // controllers can navigate independently.
        let mut sticks: std::collections::HashMap<gilrs::GamepadId, crate::input::StickState> =
            std::collections::HashMap::new();

        // Locked ~60 Hz cadence.
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(16));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;

            // Collect inputs while borrowing `backend`, then release the
            // borrow before awaiting. `Gilrs` is `Send` but not `Sync`, so a
            // `&Gilrs` (e.g. the `gamepads()` iterator) held across an await
            // would make this future non-`Send`.
            let pending = {
                let mut pending = Vec::new();

                if let Some(backend) = backend.as_mut() {
                    // 1. Drain discrete events (button presses, hot-plug).
                    while let Some(gilrs::Event { event, .. }) = backend.next_event() {
                        if let gilrs::EventType::ButtonPressed(button, _) = event
                            && let Some(message) = crate::input::gamepad_to_message(button)
                        {
                            pending.push(message);
                        }
                    }

                    // 2. Poll the continuous left-stick axes on every
                    // connected gamepad, edge-triggering navigation steps.
                    for (id, pad) in backend.gamepads() {
                        let (x, y) = (
                            pad.value(gilrs::Axis::LeftStickX),
                            pad.value(gilrs::Axis::LeftStickY),
                        );

                        if let Some(message) = sticks.entry(id).or_default().sample((x, y)) {
                            pending.push(message);
                        }

                        // The right stick drives continuous scrolling of the
                        // focused pane (edge-free, one pixel step per tick).
                        let (rx, ry) = (
                            pad.value(gilrs::Axis::RightStickX),
                            pad.value(gilrs::Axis::RightStickY),
                        );

                        if let Some(message) = crate::input::right_stick_to_scroll((rx, ry)) {
                            pending.push(message);
                        }
                    }
                }

                pending
            };

            for message in pending {
                let _ = output.send(message).await;
            }
        }
    })
}
