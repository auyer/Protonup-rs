use iced::widget::{button, center, progress_bar, text, Column, Container};
use iced::{Center, Element, Task, Subscription};
use iced::time;

use libprotonup::apps::{list_installed_apps, AppInstallations};

mod download;
mod download_task;
use download_task::{DownloadUpdate, ToolProgress, GlobalProgress};
use download::DownloadPhase;

#[cfg(test)]
mod gui_tests;

#[derive(Debug, Clone)]
enum Message {
    ScanApps,
    AppsScanned(Vec<AppInstallations>),
    StartDownload,
    DownloadUpdate(DownloadUpdate),
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
    detected_apps: Vec<AppInstallations>,
    scan_complete: bool,
    download_started: bool,
    tools: Vec<ToolDownload>,
    global_phase: DownloadPhase,
    global_status: String,
    global_progress: f32,
    download_complete: Option<Result<Vec<String>, String>>,
}

impl ProtonupGui {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanApps => Task::perform(list_installed_apps(), Message::AppsScanned),

            Message::AppsScanned(apps) => {
                self.detected_apps = apps.clone();
                self.scan_complete = true;
                
                // Pre-populate tools based on detected apps
                self.tools.clear();
                for app in &apps {
                    let compat_tool = app.as_app().default_compatibility_tool();
                    self.tools.push(ToolDownload::new(
                        compat_tool.name,
                        app.to_string(),
                    ));
                }
                
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

            Message::StartDownload => {
                self.download_started = true;
                self.global_progress = 0.0;
                self.global_phase = DownloadPhase::DetectingApps;
                self.download_complete = None;
                self.global_status = "Starting Quick Update...".to_string();
                
                // Reset all tools
                for tool in &mut self.tools {
                    tool.progress = 0.0;
                    tool.phase = DownloadPhase::DetectingApps;
                    tool.status = ToolStatus::Pending;
                }

                // Use streaming download task with logical progress
                download_task::run_quick_update(false)
                    .map(Message::DownloadUpdate)
            }

            Message::DownloadUpdate(update) => match update {
                DownloadUpdate::ToolProgress(progress) => {
                    // Find and update the specific tool
                    if let Some(tool) = self.tools.iter_mut().find(|t| t.name == progress.tool_name) {
                        tool.update_from_progress(&progress);
                        if progress.phase == DownloadPhase::Complete {
                            // Extract version from status message if available
                            if let Some(version) = progress.status_message.split(' ').nth(1) {
                                tool.version = Some(version.to_string());
                            }
                        }
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
        }
    }

    fn view(&self) -> Element<Message> {
        let mut content = Column::new().spacing(20).padding(20).align_x(Center);

        // Title
        content = content.push(text("Protonup-rs GUI").size(24));

        // Detected apps section
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

        // Download section
        if self.scan_complete && !self.detected_apps.is_empty() {
            // Start button (only show if not started)
            if !self.download_started {
                content = content.push(text("Ready to download compatibility tools").size(14));
                content = content.push(
                    button(text("Start Quick Update").size(16))
                        .on_press(Message::StartDownload)
                        .padding(10),
                );
            }

            // Per-tool progress bars (show when download started)
            if self.download_started {
                // Global status
                let phase_text = match &self.global_phase {
                    DownloadPhase::DetectingApps => "🔍 Detecting apps...",
                    DownloadPhase::FetchingReleases => "📦 Fetching releases...",
                    DownloadPhase::Downloading => "⬇️  Downloading in parallel...",
                    DownloadPhase::Validating => "✓ Validating...",
                    DownloadPhase::Unpacking => "📥 Installing...",
                    DownloadPhase::Complete => "✓ Complete!",
                    DownloadPhase::Error => "✗ Error",
                };
                content = content.push(text(phase_text).size(12));
                
                // Global progress bar
                content = content.push(
                    Column::new()
                        .spacing(5)
                        .align_x(Center)
                        .push(progress_bar(0.0..=100.0, self.global_progress))
                        .push(text(&self.global_status).size(12)),
                );

                // Per-tool progress bars
                content = content.push(text("Individual Tool Progress:").size(14));
                
                let tools_column = Column::with_children(
                    self.tools.iter().map(|tool| {
                        let status_color = match &tool.status {
                            ToolStatus::Complete => [0.3, 1.0, 0.3],
                            ToolStatus::Error(_) => [1.0, 0.3, 0.3],
                            _ => [1.0, 1.0, 1.0],
                        };
                        
                        Column::new()
                            .spacing(5)
                            .push(text(format!("{} for {}", tool.name, tool.app_target)).size(12))
                            .push(progress_bar(0.0..=100.0, tool.progress))
                            .push(text(tool.status_text()).size(10).color(status_color))
                            .into()
                    })
                )
                .spacing(15);
                
                content = content.push(tools_column);

                // Completion section
                if let Some(ref result) = self.download_complete {
                    match result {
                        Ok(versions) => {
                            content = content.push(
                                text("✓ All tools installed successfully!").color([0.3, 1.0, 0.3]),
                            );
                            
                            // Show installed versions
                            for version in versions {
                                content = content.push(text(format!("  • {}", version)).size(12));
                            }
                            
                            content = content.push(
                                button(text("Restart").size(14))
                                    .on_press(Message::ScanApps)
                                    .padding(5),
                            );
                        }
                        Err(e) => {
                            content = content.push(
                                text(format!("✗ Failed: {}", e)).color([1.0, 0.3, 0.3]),
                            );
                            content = content.push(
                                button(text("Try Again").size(14))
                                    .on_press(Message::StartDownload)
                                    .padding(5),
                            );
                        }
                    }
                }
            }
        }

        Container::new(center(content)).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if !self.scan_complete {
            time::every(std::time::Duration::from_millis(100)).map(|_| Message::ScanApps)
        } else {
            Subscription::none()
        }
    }
}

pub fn main() -> iced::Result {
    iced::application(ProtonupGui::default, ProtonupGui::update, ProtonupGui::view)
        .subscription(ProtonupGui::subscription)
        .run()
}
