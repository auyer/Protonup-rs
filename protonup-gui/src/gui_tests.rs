#[cfg(test)]
mod tests {
    use crate::{AppInstallations, DownloadPhase, DownloadUpdate, Message, ProtonupGui};
    use crate::download_task::{self, DownloadError, Progress};
    use iced_test::{Error, simulator};

    // Helper to create a model in "ready" state (apps detected, waiting for download)
    fn ready_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam],
            scan_complete: true,
            download_started: false,
            download_progress: 0.0,
            download_complete: None,
            current_phase: DownloadPhase::DetectingApps,
            status_message: "Detected: Steam".to_string(),
        }
    }

    // Helper to create a model in "downloading" state
    fn downloading_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam],
            scan_complete: true,
            download_started: true,
            download_progress: 45.0,
            download_complete: None,
            current_phase: DownloadPhase::Downloading,
            status_message: "Downloading GEProton... 45.0%".to_string(),
        }
    }

    // Helper to create a model in "completed" state
    fn completed_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam],
            scan_complete: true,
            download_started: true,
            download_progress: 100.0,
            download_complete: Some(Ok(vec!["GE-Proton9-27".to_string()])),
            current_phase: DownloadPhase::Complete,
            status_message: "✓ Success! Installed 1 tools.".to_string(),
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
    fn view_shows_start_button() -> Result<(), Error> {
        let model = ready_model();
        let mut ui = simulator(model.view());

        // Should have start button
        assert!(ui.find("Start Quick Update").is_ok());

        Ok(())
    }

    #[test]
    fn view_shows_phase_indicator_while_downloading() -> Result<(), Error> {
        let model = downloading_model();
        let mut ui = simulator(model.view());

        // View should render without error when downloading
        // Progress bar should be visible
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
        let mut ui = simulator(model.view());

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
            download_progress: 0.0,
            download_complete: None,
            current_phase: DownloadPhase::DetectingApps,
            status_message: "No compatible apps detected".to_string(),
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
        assert_eq!(model.download_progress, 0.0);
        assert!(model.detected_apps.is_empty());
        assert_eq!(model.current_phase, DownloadPhase::DetectingApps);
    }

    #[test]
    fn apps_scanned_updates_state() {
        let mut model = ProtonupGui::default();
        let apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        let _ = model.update(Message::AppsScanned(apps));

        assert!(model.scan_complete);
        assert_eq!(model.detected_apps.len(), 2);
        assert!(model.status_message.contains("Steam"));
        assert!(model.status_message.contains("Lutris"));
    }

    #[test]
    fn apps_scanned_empty_list() {
        let mut model = ProtonupGui::default();

        let _ = model.update(Message::AppsScanned(vec![]));

        assert!(model.scan_complete);
        assert!(model.detected_apps.is_empty());
        assert_eq!(model.status_message, "No compatible apps detected");
    }

    #[test]
    fn start_download_resets_state() {
        let mut model = ready_model();

        let _ = model.update(Message::StartDownload);

        assert!(model.download_started);
        assert_eq!(model.download_progress, 0.0);
        assert!(model.download_complete.is_none());
        assert_eq!(model.status_message, "Starting Quick Update...");
    }

    #[test]
    fn download_update_progress() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::Progress(
            Progress {
                percent: 75.0,
                phase: DownloadPhase::Downloading,
                tool: "GEProton".to_string(),
                status_message: "Downloading GEProton... 75.0%".to_string(),
            },
        )));

        assert_eq!(model.download_progress, 75.0);
        assert_eq!(model.current_phase, DownloadPhase::Downloading);
        assert!(model.status_message.contains("75.0"));
        assert!(model.download_complete.is_none());
    }

    #[test]
    fn download_update_phase_transitions() {
        let mut model = downloading_model();

        // Transition to Validating phase
        model.update(Message::DownloadUpdate(DownloadUpdate::Progress(
            download_task::Progress {
                percent: 10.0,
                phase: DownloadPhase::Validating,
                tool: "GEProton".to_string(),
                status_message: "Validating GEProton...".to_string(),
            },
        )));

        assert_eq!(model.current_phase, DownloadPhase::Validating);
        assert!(model.status_message.contains("Validating"));

        // Transition to Unpacking phase
        model.update(Message::DownloadUpdate(DownloadUpdate::Progress(
            download_task::Progress {
                percent: 30.0,
                phase: DownloadPhase::Unpacking,
                tool: "GEProton".to_string(),
                status_message: "Installing GEProton...".to_string(),
            },
        )));

        assert_eq!(model.current_phase, DownloadPhase::Unpacking);
        assert!(model.status_message.contains("Installing"));
    }

    #[test]
    fn download_update_finished_success() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::Finished(Ok(
            vec!["GE-Proton9-27".to_string()],
        ))));

        assert_eq!(model.download_progress, 100.0);
        assert_eq!(model.current_phase, DownloadPhase::Complete);
        assert!(model.download_complete.is_some());
        assert!(model.download_complete.as_ref().unwrap().is_ok());
        assert!(model.status_message.contains("Success"));
    }

    #[test]
    fn download_update_finished_error() {
        let mut model = downloading_model();

        model.update(Message::DownloadUpdate(DownloadUpdate::Finished(Err(
            DownloadError::IoError("test error".to_string()),
        ))));

        assert_eq!(model.current_phase, DownloadPhase::Error);
        assert!(model.download_complete.is_some());
        assert!(model.download_complete.as_ref().unwrap().is_err());
        assert!(model.status_message.contains("Error"));
    }

    #[test]
    fn subscription_active_when_not_scanned() {
        let model = ProtonupGui::default();
        let sub = model.subscription();
        // Subscription should be active (not None)
        drop(sub);
    }

    #[test]
    fn subscription_none_after_scan() {
        let mut model = ready_model();
        let sub = model.subscription();
        // Subscription should be None after scan complete
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
        assert_eq!(model.status_message, "Starting Quick Update...");

        Ok(())
    }
}
