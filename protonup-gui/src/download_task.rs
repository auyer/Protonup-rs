//! Task::sip() wrapper for the download module.
//! 
//! This module provides the bridge between the download logic in download.rs
//! and Iced's Task::sip() pattern for streaming updates to the GUI.

use iced::task::sipper;
use iced::Task;

use crate::download::{self, DownloadPhase};

/// Progress updates streamed from the download task to the GUI
#[derive(Debug, Clone)]
pub enum DownloadUpdate {
    Progress(Progress),
    Finished(Result<Vec<String>, DownloadError>),
}

/// Progress information for the GUI
#[derive(Debug, Clone)]
pub struct Progress {
    pub percent: f32,
    pub phase: DownloadPhase,
    pub tool: String,
    pub status_message: String,
}

/// Internal progress type for sipper
#[derive(Debug, Clone)]
pub(crate) struct SipProgress {
    pub percent: f32,
    pub phase: DownloadPhase,
    pub tool: String,
    pub status_message: String,
}

impl SipProgress {
    pub fn new(phase: DownloadPhase, tool: &str, status_message: &str, percent: f32) -> Self {
        Self {
            percent,
            phase,
            tool: tool.to_string(),
            status_message: status_message.to_string(),
        }
    }
}

/// Download errors
#[derive(Debug, Clone)]
pub enum DownloadError {
    IoError(String),
    ValidationError(String),
    UnpackError(String),
    NoAppsFound,
}

/// Creates a streaming task that runs quick downloads and reports progress
/// 
/// This uses Iced's `Task::sip()` pattern where:
/// - The sipper async closure runs the download logic and sends progress updates
/// - Progress updates are mapped to DownloadUpdate::Progress
/// - Final result is mapped to DownloadUpdate::Finished
pub fn run_quick_update(force: bool) -> Task<DownloadUpdate> {
    // Create the sipper straw that runs the download logic
    let straw = sipper(async move |mut progress_sender| {
        // Run the actual download logic with progress callback
        // We use a synchronous channel to forward progress from the callback to the sipper
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SipProgress>();
        
        // Spawn a task that forwards progress from the channel to the sipper
        let forward_task = tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                let _ = progress_sender.send(progress).await;
            }
        });
        
        // Run the download with the channel sender as callback
        let result = download::run_with_progress_callback(
            move |progress: SipProgress| {
                let _ = tx.send(progress);
            },
            force,
        ).await;
        
        // Clean up the forward task
        forward_task.abort();
        
        // Map result to Ok/Err
        match result {
            Ok(releases) => {
                let versions: Vec<String> = releases.iter().map(|r| r.tag_name.clone()).collect();
                Ok(versions)
            }
            Err(e) => Err(DownloadError::IoError(e.to_string())),
        }
    });
    
    // Wrap in Task::sip with progress and result mapping
    let (task, handle) = Task::sip(
        straw,
        // Progress callback - receives progress from sipper
        |sip_progress: SipProgress| {
            DownloadUpdate::Progress(Progress {
                percent: sip_progress.percent,
                phase: sip_progress.phase,
                tool: sip_progress.tool,
                status_message: sip_progress.status_message,
            })
        },
        // Transform the final result into DownloadUpdate::Finished
        |result| DownloadUpdate::Finished(result),
    )
    .abortable();
    
    // Drop handle to auto-cancel when task is dropped
    drop(handle);
    task
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_update_is_clone() {
        let u = DownloadUpdate::Progress(Progress {
            percent: 25.0,
            phase: DownloadPhase::Downloading,
            tool: "GEProton".to_string(),
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
    fn progress_is_clone() {
        let p = Progress {
            percent: 50.0,
            phase: DownloadPhase::Validating,
            tool: "WineGE".to_string(),
            status_message: "Validating...".to_string(),
        };
        let _p2 = p.clone();
    }

    #[test]
    fn download_phase_is_clone() {
        let phase = DownloadPhase::Unpacking;
        let _phase2 = phase.clone();
    }

    #[test]
    fn sip_progress_is_clone() {
        let p = SipProgress::new(
            DownloadPhase::Downloading,
            "GEProton",
            "Downloading...",
            50.0,
        );
        let _p2 = p.clone();
    }
}
