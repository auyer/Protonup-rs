use iced::widget::{button, center, progress_bar, text, Column, Container};
use iced::{Center, Element, Task, Subscription};
use iced::time;

use libprotonup::apps::{list_installed_apps, AppInstallations};

mod download;
mod download_task;
use download_task::DownloadUpdate;
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

#[derive(Debug, Default)]
struct ProtonupGui {
    detected_apps: Vec<AppInstallations>,
    scan_complete: bool,
    download_started: bool,
    download_progress: f32,
    download_complete: Option<Result<Vec<String>, String>>,
    current_phase: DownloadPhase,
    status_message: String,
}

impl ProtonupGui {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScanApps => Task::perform(list_installed_apps(), Message::AppsScanned),

            Message::AppsScanned(apps) => {
                self.detected_apps = apps;
                self.scan_complete = true;
                if self.detected_apps.is_empty() {
                    self.status_message = "No compatible apps detected".to_string();
                } else {
                    self.status_message = format!(
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
                self.download_progress = 0.0;
                self.download_complete = None;
                self.current_phase = DownloadPhase::DetectingApps;
                self.status_message = "Starting Quick Update...".to_string();

                // Use streaming download task with logical progress
                download_task::run_quick_update(false)
                    .map(Message::DownloadUpdate)
            }

            Message::DownloadUpdate(update) => match update {
                DownloadUpdate::Progress(progress) => {
                    self.download_progress = progress.percent;
                    self.current_phase = progress.phase.clone();
                    self.status_message = progress.status_message;
                    Task::none()
                }
                DownloadUpdate::Finished(result) => {
                    match result {
                        Ok(versions) => {
                            self.download_progress = 100.0;
                            self.current_phase = DownloadPhase::Complete;
                            self.status_message = format!(
                                "✓ Success! Installed {} tools.",
                                versions.len()
                            );
                            self.download_complete = Some(Ok(versions));
                        }
                        Err(e) => {
                            self.current_phase = DownloadPhase::Error;
                            self.status_message = format!("✗ Error: {:?}", e);
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
                text(&self.status_message)
            };
            content = content.push(apps_text.size(14));
        } else {
            content = content.push(text(&self.status_message).size(14));
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

            // Progress section (show when download started)
            if self.download_started {
                // Phase indicator
                let phase_text = match &self.current_phase {
                    DownloadPhase::DetectingApps => "🔍 Detecting apps...",
                    DownloadPhase::FetchingReleases => "📦 Fetching releases...",
                    DownloadPhase::Downloading => "⬇️  Downloading...",
                    DownloadPhase::Validating => "✓ Validating...",
                    DownloadPhase::Unpacking => "📥 Installing...",
                    DownloadPhase::Complete => "✓ Complete!",
                    DownloadPhase::Error => "✗ Error",
                };
                content = content.push(text(phase_text).size(12));

                // Progress bar
                content = content.push(
                    Column::new()
                        .spacing(10)
                        .align_x(Center)
                        .push(progress_bar(0.0..=100.0, self.download_progress))
                        .push(text(&self.status_message)),
                );

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
