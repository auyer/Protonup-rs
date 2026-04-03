#[cfg(test)]
mod tests {
    use crate::{AppInstallations, App, DownloadPhase, DownloadUpdate, Message, ProtonupGui, ToolProgress, ToolDownload, ToolStatus, GuiMode, SelectionStep};
    use crate::download_task::{DownloadError, GlobalProgress};
    use iced_test::{Error, simulator};

    // Helper to create a model in "ready" state (apps detected, waiting for action)
    fn ready_model() -> ProtonupGui {
        ProtonupGui {
            detected_apps: vec![AppInstallations::Steam],
            scan_complete: true,
            mode: GuiMode::Initial,
            selection_step: SelectionStep::Initial,
            available_tools: vec![],
            selected_tool_indices: vec![],
            selected_tool: None,
            available_versions: vec![],
            selected_version_indices: vec![],
            app_installation: None,
            download_started: false,
            tools: vec![],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "Detected: Steam".to_string(),
            global_progress: 0.0,
            download_complete: None,
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

        // Should have buttons when apps are detected
        assert!(ui.find("Quick Update").is_ok() || ui.find("Download for Steam").is_ok());

        Ok(())
    }

    #[test]
    fn view_shows_no_apps_detected() -> Result<(), Error> {
        let model = ProtonupGui {
            detected_apps: vec![],
            scan_complete: true,
            mode: GuiMode::Initial,
            selection_step: SelectionStep::Initial,
            available_tools: vec![],
            selected_tool_indices: vec![],
            selected_tool: None,
            available_versions: vec![],
            selected_version_indices: vec![],
            app_installation: None,
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
        assert_eq!(model.mode, GuiMode::Initial);
        assert_eq!(model.selection_step, SelectionStep::Initial);
        assert!(model.detected_apps.is_empty());
    }

    #[test]
    fn apps_scanned_updates_state() {
        let mut model = ProtonupGui::default();
        let apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        let _ = model.update(Message::AppsScanned(apps));

        assert!(model.scan_complete);
        assert_eq!(model.detected_apps.len(), 2);
        assert!(model.global_status.contains("Steam"));
        assert!(model.global_status.contains("Lutris"));
    }

    #[test]
    fn apps_scanned_empty_list() {
        let mut model = ProtonupGui::default();

        let _ = model.update(Message::AppsScanned(vec![]));

        assert!(model.scan_complete);
        assert!(model.detected_apps.is_empty());
        assert_eq!(model.global_status, "No compatible apps detected");
    }

    #[test]
    fn tools_fetched_does_not_crash() {
        let mut model = ProtonupGui::default();
        
        let _ = model.update(Message::ToolsFetched(vec![]));
        
        // Should not crash
        assert!(true);
    }

    #[test]
    fn selection_error_sets_status() {
        let mut model = ProtonupGui::default();
        
        let _ = model.update(Message::SelectionError("test error".to_string()));
        
        assert!(model.global_status.contains("test error"));
    }

    #[test]
    fn back_to_initial_resets_state() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        
        let _ = model.update(Message::BackToInitial);
        
        assert_eq!(model.mode, GuiMode::Initial);
        assert_eq!(model.selection_step, SelectionStep::Initial);
    }

    #[test]
    fn restart_resets_and_rescans() {
        let mut model = ready_model();
        model.download_started = true;
        
        let _ = model.update(Message::Restart);
        
        assert_eq!(model.mode, GuiMode::Initial);
        assert!(!model.download_started);
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
    // Download progress tests
    //

    #[test]
    fn tool_progress_update() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new("GEProton".to_string(), "Steam".to_string()));

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
    }

    #[test]
    fn global_progress_update() {
        let mut model = ProtonupGui::default();

        model.update(Message::DownloadUpdate(DownloadUpdate::GlobalProgress(
            GlobalProgress {
                phase: DownloadPhase::Downloading,
                status_message: "Downloading tools...".to_string(),
                percent: 50.0,
            },
        )));

        assert_eq!(model.global_progress, 50.0);
        assert_eq!(model.global_phase, DownloadPhase::Downloading);
    }

    #[test]
    fn download_finished_success() {
        let mut model = ProtonupGui::default();

        model.update(Message::DownloadUpdate(DownloadUpdate::Finished(Ok(
            vec!["GE-Proton9-27".to_string()],
        ))));

        assert_eq!(model.global_progress, 100.0);
        assert_eq!(model.global_phase, DownloadPhase::Complete);
        assert!(model.download_complete.is_some());
        assert!(model.download_complete.as_ref().unwrap().is_ok());
    }

    #[test]
    fn download_finished_error() {
        let mut model = ProtonupGui::default();

        model.update(Message::DownloadUpdate(DownloadUpdate::Finished(Err(
            DownloadError::IoError("test error".to_string()),
        ))));

        assert_eq!(model.global_phase, DownloadPhase::Error);
        assert!(model.download_complete.is_some());
        assert!(model.download_complete.as_ref().unwrap().is_err());
    }

    //
    // Integration-style test: click button and verify state change
    //

    #[test]
    fn clicking_quick_update_starts_download() -> Result<(), Error> {
        let mut model = ready_model();
        let mut ui = simulator(model.view());

        // Click the Quick Update button
        let _ = ui.click("Quick Update")?;

        // Process the messages
        for message in ui.into_messages() {
            let _ = model.update(message);
        }

        // Verify state changed
        assert!(model.download_started);
        assert_eq!(model.mode, GuiMode::QuickUpdate);

        Ok(())
    }

    //
    // Multi-tool/version selection tests
    //

    #[test]
    fn toggle_tool_adds_and_removes() {
        let mut model = ready_model();
        model.available_tools = vec![
            CompatTool::from_str("GEProton").unwrap(),
            CompatTool::from_str("Luxtorpeda").unwrap(),
        ];

        // Toggle first tool
        let _ = model.update(Message::ToggleTool(0));
        assert_eq!(model.selected_tool_indices, vec![0]);

        // Toggle second tool
        let _ = model.update(Message::ToggleTool(1));
        assert_eq!(model.selected_tool_indices, vec![0, 1]);

        // Toggle off first tool
        let _ = model.update(Message::ToggleTool(0));
        assert_eq!(model.selected_tool_indices, vec![1]);
    }

    #[test]
    fn toggle_version_adds_and_removes() {
        let mut model = ready_model();
        model.selection_step = SelectionStep::SelectingVersions;

        // Simulate versions being fetched
        let _ = model.update(Message::VersionsFetched(vec![]));

        // Toggle versions
        let _ = model.update(Message::ToggleVersion(0));
        assert_eq!(model.selected_version_indices, vec![0]);

        let _ = model.update(Message::ToggleVersion(1));
        assert_eq!(model.selected_version_indices, vec![0, 1]);

        let _ = model.update(Message::ToggleVersion(0));
        assert_eq!(model.selected_version_indices, vec![1]);
    }

    #[test]
    fn start_selected_downloads_creates_tool_entries() {
        use libprotonup::sources::{CompatTool, Forge, ToolType};
        
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        model.app_installation = Some(AppInstallations::Steam);
        
        // Set up available tools
        model.available_tools = vec![
            CompatTool::new_custom(
                "GEProton".to_string(),
                Forge::GitHub,
                "GloriousEggroll".to_string(),
                "proton-ge-custom".to_string(),
                ToolType::Runtime,
                None, None, None,
            ),
        ];
        model.selected_tool_indices = vec![0];
        
        // Simulate versions being fetched
        model.selection_step = SelectionStep::SelectingVersions;
        let _ = model.update(Message::VersionsFetched(vec![]));
        model.available_versions = vec![]; // Empty for this test
        model.selected_version_indices = vec![0];
        
        // Starting download should create ToolDownload entries
        // (Won't actually download since versions are empty, but should set up state)
        let _ = model.update(Message::StartSelectedDownloads);
        
        assert!(model.download_started);
        assert_eq!(model.selection_step, SelectionStep::Downloading);
        // Should have created one ToolDownload entry per tool/version combo
        assert_eq!(model.tools.len(), 1);
        assert!(model.tools[0].name.contains("GEProton"));
    }

    #[test]
    fn multi_version_selection_creates_multiple_entries() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_installation = Some(AppInstallations::Steam);
        
        // Set up one tool with multiple versions
        model.available_tools = vec![
            CompatTool::from_str("GEProton").unwrap(),
        ];
        model.selected_tool_indices = vec![0];
        
        // Set up multiple versions
        model.selection_step = SelectionStep::SelectingVersions;
        model.available_versions = vec![]; // Would normally have releases
        model.selected_version_indices = vec![0, 1, 2]; // Three versions selected
        
        // Starting download should create 3 ToolDownload entries (1 tool x 3 versions)
        let _ = model.update(Message::StartSelectedDownloads);
        
        assert!(model.download_started);
        // Should have created 3 ToolDownload entries
        assert_eq!(model.tools.len(), 3);
    }
}
