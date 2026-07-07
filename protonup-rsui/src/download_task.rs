use iced::Task;
use iced::task::{self, sipper};
use std::collections::HashSet;
use std::future::Future;

use libprotonup::apps::AppInstallations;
use libprotonup::downloads::Release;
use libprotonup::sources::CompatTool;
use tokio::sync::mpsc::UnboundedSender;

use crate::download::{self, DownloadPhase};

/// Progress updates streamed from the download task to the GUI
#[derive(Debug, Clone)]
pub enum DownloadUpdate {
    ToolProgress(ToolProgress),
    GlobalProgress(GlobalProgress),
    Finished(Result<Vec<String>, DownloadError>),
}

/// Per-tool progress information
#[derive(Debug, Clone)]
pub struct ToolProgress {
    pub tool_name: String,
    pub phase: DownloadPhase,
    pub percent: f32,
    pub status_message: String,
}

/// Global progress information (overall status)
#[derive(Debug, Clone)]
pub struct GlobalProgress {
    pub phase: DownloadPhase,
    pub status_message: String,
    pub percent: f32,
}

/// Internal progress type for sipper
#[derive(Debug, Clone)]
pub(crate) struct SipProgress {
    pub tool_name: Option<String>,
    pub phase: DownloadPhase,
    pub percent: f32,
    pub status_message: String,
}

impl SipProgress {
    pub fn new(phase: DownloadPhase, tool_name: &str, status_message: &str, percent: f32) -> Self {
        Self {
            percent,
            phase,
            tool_name: if tool_name.is_empty() {
                None
            } else {
                Some(tool_name.to_string())
            },
            status_message: status_message.to_string(),
        }
    }

    pub fn global(phase: DownloadPhase, status_message: &str, percent: f32) -> Self {
        Self {
            percent,
            phase,
            tool_name: None,
            status_message: status_message.to_string(),
        }
    }
}

/// Download errors
#[derive(Debug, Clone)]
pub enum DownloadError {
    IoError(String),
}

fn sip_task<F, Fut>(download_fn: F) -> (Task<DownloadUpdate>, task::Handle)
where
    F: FnOnce(UnboundedSender<SipProgress>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<Vec<String>, DownloadError>> + Send + 'static,
{
    let straw = sipper(async move |mut progress_sender| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SipProgress>();

        let forward_task = tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                let _ = progress_sender.send(progress).await;
            }
        });

        let result = download_fn(tx).await;

        forward_task.abort();

        result
    });

    let (task, handle) = Task::sip(
        straw,
        |sip_progress: SipProgress| {
            if let Some(tool_name) = sip_progress.tool_name {
                DownloadUpdate::ToolProgress(ToolProgress {
                    tool_name,
                    phase: sip_progress.phase,
                    percent: sip_progress.percent,
                    status_message: sip_progress.status_message,
                })
            } else {
                DownloadUpdate::GlobalProgress(GlobalProgress {
                    phase: sip_progress.phase,
                    status_message: sip_progress.status_message,
                    percent: sip_progress.percent,
                })
            }
        },
        DownloadUpdate::Finished,
    )
    .abortable();

    (task, handle)
}

/// Creates a streaming task that downloads selected tools and versions
///
/// Returns a tuple of (Task, Handle) where the Handle can be used to abort the task
pub fn run_quick_update(force: bool) -> (Task<DownloadUpdate>, task::Handle) {
    sip_task(move |tx| async move {
        let result = download::run_with_progress_callback(
            move |progress: SipProgress| {
                let _ = tx.send(progress);
            },
            force,
        )
        .await;

        match result {
            Ok(releases) => {
                let versions: Vec<String> = releases.iter().map(|r| r.tag_name.clone()).collect();
                Ok(versions)
            }
            Err(e) => Err(DownloadError::IoError(e.to_string())),
        }
    })
}

pub fn download_selected_tools(
    app_installation: AppInstallations,
    tools_and_versions: Vec<(CompatTool, Vec<Release>)>,
    force_reinstall_names: HashSet<String>,
    arch_variant: Option<u8>,
) -> (Task<DownloadUpdate>, task::Handle) {
    sip_task(move |tx| async move {
        let result = download::download_selected_tools(
            app_installation,
            tools_and_versions,
            move |progress: SipProgress| {
                let _ = tx.send(progress);
            },
            force_reinstall_names,
            arch_variant,
        )
        .await;

        match result {
            Ok(releases) => {
                let versions: Vec<String> = releases.iter().map(|r| r.tag_name.clone()).collect();
                Ok(versions)
            }
            Err(e) => Err(DownloadError::IoError(e.to_string())),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_update_is_clone() {
        let u = DownloadUpdate::ToolProgress(ToolProgress {
            tool_name: "GEProton".to_string(),
            phase: DownloadPhase::Downloading,
            percent: 25.0,
            status_message: "Downloading...".to_string(),
        });
        let _u2 = u.clone();
    }

    #[test]
    fn download_error_is_clone() {
        let e = DownloadError::IoError("test".to_string());
        let _e2 = e.clone();
    }

    #[test]
    fn tool_progress_is_clone() {
        let p = ToolProgress {
            tool_name: "WineGE".to_string(),
            phase: DownloadPhase::Validating,
            percent: 50.0,
            status_message: "Validating...".to_string(),
        };
        let _p2 = p.clone();
    }

    #[test]
    fn global_progress_is_clone() {
        let p = GlobalProgress {
            phase: DownloadPhase::Downloading,
            status_message: "Downloading...".to_string(),
            percent: 50.0,
        };
        let _p2 = p.clone();
    }

    #[test]
    fn download_phase_is_clone() {
        let phase = DownloadPhase::Unpacking;
        let _phase2 = phase.clone();
    }

    #[test]
    fn sip_progress_tool_is_clone() {
        let p = SipProgress::new(
            DownloadPhase::Downloading,
            "GEProton",
            "Downloading...",
            50.0,
        );
        let _p2 = p.clone();
    }

    #[test]
    fn sip_progress_global_is_clone() {
        let p = SipProgress::global(DownloadPhase::FetchingReleases, "Fetching...", 10.0);
        let _p2 = p.clone();
    }
}
