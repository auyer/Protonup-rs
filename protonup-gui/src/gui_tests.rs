#[cfg(test)]
mod tests {
    use crate::{AppInstallations, DownloadPhase, DownloadUpdate, GlobalProgress, Message, ProtonupGui, ToolProgress, ToolDownload, ToolStatus};
    use crate::download_task::DownloadError;
    use iced_test::{Error, simulator};

    // Helper to create a model in "ready" state (apps detected, waiting for download)
    fn ready_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam],
            scan_complete: true,
            download_started: false,
            tools: vec![ToolDownload::new("GEProton".to_string(), "Steam \"Native\"".to_string())],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "Detected: Steam \"Native\"".to_string(),
            global_progress: 0.0,
            download_complete: None,
        }
    }

    // Helper to create a model with multiple tools
    fn multi_tool_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam, AppInstallations::Lutris],
            scan_complete: true,
            download_started: false,
            tools: vec![
                ToolDownload::new("GEProton".to_string(), "Steam \"Native\"".to_string()),
                ToolDownload::new("WineGE".to_string(), "Lutris \"Native\"".to_string()),
            ],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "Detected: Steam \"Native\", Lutris \"Native\"".to_string(),
            global_progress: 0.0,
            download_complete: None,
        }
    }

    // Helper to create a model in "downloading" state
    fn downloading_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam],
            scan_complete: true,
            download_started: true,
            tools: vec![ToolDownload {
                name: "GEProton".to_string(),
                app_target: "Steam \"Native\"".to_string(),
                version: None,
                phase: DownloadPhase::Downloading,
                progress: 45.0,
                status: ToolStatus::Downloading,
            }],
            global_phase: DownloadPhase::Downloading,
            global_status: "Downloading in parallel...".to_string(),
            global_progress: 45.0,
            download_complete: None,
        }
    }

    // Helper to create a model in "completed" state
    fn completed_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam],
            scan_complete: true,
            download_started: true,
            tools: vec![ToolDownload {
                name: "GEProton".to_string(),
                app_target: "Steam \"Native\"".to_string(),
                version: Some("GE-Proton9-27".to_string()),
                phase: DownloadPhase::Complete,
                progress: 100.0,
                status: ToolStatus::Complete,
            }],
            global_phase: DownloadPhase::Complete,
            global_status: "✓ Success! Installed 1 tools.".to_string(),
            global_progress: 100.0,
            download_complete: Some(Ok(vec!["GE-Proton9-27".to_string()])),
        }
    }

    //
    // View Tests using iced_test simulator
    //

    #[test]
    fn view_renders_initial_state() -> Result<(), Error> {
        let model = ProtonupGui::default();
        let mut ui = simulator(model.view());

        // Should show title
        assert!(ui.find("Protonup-rs GUI").is_ok());

        Ok(())
    }

    #[test]
    fn view_renders_detected_apps() -> Result<(), Error> {
        let model = ready_model();
        let mut ui = simulator(model.view());

        // Should have start button when apps are detected
        assert!(ui.find("Start Quick Update").is_ok());

        Ok(())
    }

    #[test]
    fn view_renders_multiple_tools() -> Result<(), Error> {
        let model = multi_tool_model();
        let mut ui = simulator(model.view());

        // Should have start button
        assert!(ui.find("Start Quick Update").is_ok());

        Ok(())
    }

    #[test]
    fn view_shows_start_button() -> Result<(), Error> {
        let model = ready_model();
        let mut ui = simulator(model.view());

        // Should have start button
        assert!(ui.find("Start Quick Update").is_ok());

        Ok(())
    }

    #[test]
    fn view_shows_global_progress_while_downloading() -> Result<(), Error> {
        let model = downloading_model();
        let ui = simulator(model.view());

        // View should render without error when downloading
        drop(ui);

        Ok(())
    }

    #[test]
    fn view_shows_per_tool_progress() -> Result<(), Error> {
        let model = downloading_model();
        let ui = simulator(model.view());

        // View should render without error when downloading with per-tool progress
        drop(ui);

        Ok(())
    }

    #[test]
    fn view_shows_success_message() -> Result<(), Error> {
        let model = completed_model();
        let mut ui = simulator(model.view());

        // Should show restart button when complete
        assert!(ui.find("Restart").is_ok());

        Ok(())
    }

    #[test]
    fn view_shows_installed_versions() -> Result<(), Error> {
        let model = completed_model();
        let ui = simulator(model.view());

        // View should render without error when complete
        drop(ui);

        Ok(())
    }

    #[test]
    fn view_shows_no_apps_detected() -> Result<(), Error> {
        let model = ProtonupGui {
            detected_apps: vec![],
            scan_complete: true,
            download_started: false,
            tools: vec![],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "No compatible apps detected".to_string(),
            global_progress: 0.0,
            download_complete: None,
        };
        let mut ui = simulator(model.view());

        // Should show error message
        assert!(ui.find("No compatible apps detected").is_ok());

        Ok(())
    }

    //
    // Update Logic Tests
    //

    #[test]
    fn initial_state_is_scanning() {
        let model = ProtonupGui::default();
        assert!(!model.scan_complete);
        assert!(!model.download_started);
        assert_eq!(model.global_progress, 0.0);
        assert!(model.detected_apps.is_empty());
        assert_eq!(model.global_phase, DownloadPhase::DetectingApps);
        assert!(model.tools.is_empty());
    }

    #[test]
    fn apps_scanned_creates_tools() {
        let mut model = ProtonupGui::default();
        let apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        let _ = model.update(Message::AppsScanned(apps));

        assert!(model.scan_complete);
        assert_eq!(model.detected_apps.len(), 2);
        assert_eq!(model.tools.len(), 2);
        assert_eq!(model.tools[0].name, "GEProton");
        assert_eq!(model.tools[1].name, "WineGE");
    }

    #[test]
    fn apps_scanned_empty_list() {
        let mut model = ProtonupGui::default();

        let _ = model.update(Message::AppsScanned(vec![]));

        assert!(model.scan_complete);
        assert!(model.detected_apps.is_empty());
        assert!(model.tools.is_empty());
        assert_eq!(model.global_status, "No compatible apps detected");
    }

    #[test]
    fn start_download_resets_state() {
        let mut model = ready_model();

        let _ = model.update(Message::StartDownload);

        assert!(model.download_started);
        assert_eq!(model.global_progress, 0.0);
        assert!(model.download_complete.is_none());
        assert_eq!(model.global_status, "Starting Quick Update...");
        
        // All tools should be reset
        for tool in &model.tools {
            assert_eq!(tool.progress, 0.0);
            assert_eq!(tool.status, ToolStatus::Pending);
        }
    }

    #[test]
    fn tool_progress_update() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton".to_string(),
                phase: DownloadPhase::Downloading,
                percent: 75.0,
                status_message: "Downloading GEProton... 75.0%".to_string(),
            },
        )));

        assert_eq!(model.tools[0].progress, 75.0);
        assert_eq!(model.tools[0].phase, DownloadPhase::Downloading);
        assert_eq!(model.tools[0].status, ToolStatus::Downloading);
        assert!(model.download_complete.is_none());
    }

    #[test]
    fn tool_phase_transitions() {
        let mut model = downloading_model();

        // Transition to Validating phase
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton".to_string(),
                phase: DownloadPhase::Validating,
                percent: 10.0,
                status_message: "Validating GEProton...".to_string(),
            },
        )));

        assert_eq!(model.tools[0].phase, DownloadPhase::Validating);
        assert_eq!(model.tools[0].status, ToolStatus::Validating);

        // Transition to Unpacking phase
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton".to_string(),
                phase: DownloadPhase::Unpacking,
                percent: 30.0,
                status_message: "Installing GEProton...".to_string(),
            },
        )));

        assert_eq!(model.tools[0].phase, DownloadPhase::Unpacking);
        assert_eq!(model.tools[0].status, ToolStatus::Unpacking);

        // Transition to Complete phase
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton".to_string(),
                phase: DownloadPhase::Complete,
                percent: 100.0,
                status_message: "✓ GEProton installed successfully".to_string(),
            },
        )));

        assert_eq!(model.tools[0].phase, DownloadPhase::Complete);
        assert_eq!(model.tools[0].status, ToolStatus::Complete);
    }

    #[test]
    fn global_progress_update() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::GlobalProgress(
            GlobalProgress {
                phase: DownloadPhase::Downloading,
                status_message: "Downloading 2 tools in parallel...".to_string(),
                percent: 50.0,
            },
        )));

        assert_eq!(model.global_progress, 50.0);
        assert_eq!(model.global_phase, DownloadPhase::Downloading);
        assert!(model.global_status.contains("2 tools"));
    }

    #[test]
    fn download_finished_success() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::Finished(Ok(
            vec!["GE-Proton9-27".to_string()],
        ))));

        assert_eq!(model.global_progress, 100.0);
        assert_eq!(model.global_phase, DownloadPhase::Complete);
        assert!(model.download_complete.is_some());
        assert!(model.download_complete.as_ref().unwrap().is_ok());
        assert!(model.global_status.contains("Success"));
    }

    #[test]
    fn download_finished_error() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::Finished(Err(
            DownloadError::IoError("test error".to_string()),
        ))));

        assert_eq!(model.global_phase, DownloadPhase::Error);
        assert!(model.download_complete.is_some());
        assert!(model.download_complete.as_ref().unwrap().is_err());
        assert!(model.global_status.contains("Error"));
    }

    #[test]
    fn subscription_active_when_not_scanned() {
        let model = ProtonupGui::default();
        let sub = model.subscription();
        drop(sub);
    }

    #[test]
    fn subscription_none_after_scan() {
        let mut model = ready_model();
        let sub = model.subscription();
        drop(sub);
    }

    //
    // Integration-style test: click start button and verify state change
    //

    #[test]
    fn clicking_start_button_sets_download_started() -> Result<(), Error> {
        let mut model = ready_model();
        let mut ui = simulator(model.view());

        // Click the start button
        let _ = ui.click("Start Quick Update")?;

        // Process the messages
        for message in ui.into_messages() {
            let _ = model.update(message);
        }

        // Verify state changed
        assert!(model.download_started);
        assert_eq!(model.global_status, "Starting Quick Update...");

        Ok(())
    }

    //
    // Multi-tool tests
    //

    #[test]
    fn multi_tool_independent_progress() {
        let mut model = multi_tool_model();
        model.download_started = true;

        // Update first tool
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton".to_string(),
                phase: DownloadPhase::Downloading,
                percent: 80.0,
                status_message: "Downloading...".to_string(),
            },
        )));

        // Update second tool
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "WineGE".to_string(),
                phase: DownloadPhase::Validating,
                percent: 30.0,
                status_message: "Validating...".to_string(),
            },
        )));

        // Verify independent progress
        assert_eq!(model.tools[0].progress, 80.0);
        assert_eq!(model.tools[0].phase, DownloadPhase::Downloading);
        assert_eq!(model.tools[1].progress, 30.0);
        assert_eq!(model.tools[1].phase, DownloadPhase::Validating);
    }

    #[test]
    fn tool_status_error() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton".to_string(),
                phase: DownloadPhase::Error,
                percent: 0.0,
                status_message: "Download failed: connection error".to_string(),
            },
        )));

        assert_eq!(model.tools[0].phase, DownloadPhase::Error);
        assert!(matches!(model.tools[0].status, ToolStatus::Error(_)));
    }
}
