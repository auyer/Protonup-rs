use iced::task;
use iced::widget::button;
use iced::{Border, Color, Task};
use std::collections::HashSet;
use std::path::PathBuf;

use libprotonup::apps::{App, AppInstallations};
use libprotonup::files;
use libprotonup::sources::CompatTool;

use crate::download::DownloadPhase;
use crate::download_task;
use crate::message::{
    AppInstallationView, AppMode, GuiMode, Message, QuickUpdateStatus, SelectionStep, ToolDownload,
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

    pub selected_arch_variant: Option<u8>,
    pub has_variant_tools: bool,

    pub app_installation: Option<AppInstallations>,

    pub already_installed_tools: Vec<ToolDownload>,
    pub force_reinstall_indices: Vec<usize>,

    pub download_started: bool,
    pub tools: Vec<ToolDownload>,
    pub global_phase: DownloadPhase,
    pub global_status: String,
    pub global_progress: f32,
    pub download_complete: Option<Result<Vec<String>, String>>,

    pub download_handle: Option<task::Handle>,

    pub app_mode: AppMode,

    pub custom_path_input: String,
    pub path_error: Option<String>,

    pub app_installations_views: Vec<AppInstallationView>,
    pub manage_status: String,
    pub manage_error: Option<String>,

    pub quick_update_status: QuickUpdateStatus,

    pub logo_handle: iced::widget::image::Handle,
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
            download_complete: None,
            download_handle: None,
            app_mode: AppMode::default(),
            custom_path_input: String::new(),
            path_error: None,
            app_installations_views: Vec::new(),
            manage_status: String::new(),
            manage_error: None,
            quick_update_status: QuickUpdateStatus::default(),
            logo_handle: iced::widget::image::Handle::from_bytes(LOGO_BYTES),
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
    ) -> Vec<(String, bool)> {
        let mut results = Vec::new();

        for app_inst in &detected_apps {
            let compat_tool = app_inst.as_app().default_compatibility_tool();
            let tool_name = compat_tool.name.clone();

            let is_installed = match app_inst.list_installed_versions().await {
                Ok(versions) => versions.iter().any(|folder| {
                    let name = &folder.0.1;
                    name.starts_with("GE-Proton") || name.starts_with("Proton-")
                }),
                Err(_) => false,
            };

            results.push((tool_name, is_installed));
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
        {
            subscriptions
                .push(iced::window::frames().map(|_| crate::message::Message::TickSpinner));
        }

        iced::Subscription::batch(subscriptions)
    }
}
