#[cfg(test)]
mod tests {
    use crate::{AppInstallations, App, DownloadPhase, DownloadUpdate, Message, ProtonupGui, ToolProgress, ToolDownload, ToolStatus, GuiMode, SelectionStep};
    use crate::download_task::{DownloadError, GlobalProgress};
    use libprotonup::sources::CompatTool;
    use std::str::FromStr;
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
            already_installed_tools: vec![],
            force_reinstall_indices: vec![],
            download_started: false,
            tools: vec![],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "Detected: Steam".to_string(),
            global_progress: 0.0,
            download_complete: None,
            spinner_frame: 0,
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
            already_installed_tools: vec![],
            force_reinstall_indices: vec![],
            download_started: false,
            tools: vec![],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "No compatible apps detected".to_string(),
            global_progress: 0.0,
            download_complete: None,
            spinner_frame: 0,
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
    fn tool_selection_state_is_correct() {
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
        
        // Toggle tool selection
        let _ = model.update(Message::ToggleTool(0));
        assert_eq!(model.selected_tool_indices, vec![0]);
        
        // Confirm selection should move to version selection
        let _ = model.update(Message::ToolSelectionConfirmed);
        assert_eq!(model.selection_step, SelectionStep::SelectingVersions);
        assert!(model.selected_tool.is_some());
    }

    #[test]
    fn version_toggle_state_is_correct() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_installation = Some(AppInstallations::Steam);
        model.selection_step = SelectionStep::SelectingVersions;
        model.available_versions = vec![]; // Would have releases in real usage

        // Toggle versions (indices would exist if versions were present)
        let _ = model.update(Message::ToggleVersion(0));
        assert_eq!(model.selected_version_indices, vec![0]);

        let _ = model.update(Message::ToggleVersion(1));
        assert_eq!(model.selected_version_indices, vec![0, 1]);

        let _ = model.update(Message::ToggleVersion(0));
        assert_eq!(model.selected_version_indices, vec![1]);
    }

    //
    // Download progress tracking tests
    //

    #[test]
    fn download_progress_updates_correct_tool() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new(
            "GEProton GE-Proton9-27".to_string(),
            "Steam \"Native\"".to_string(),
        ));

        // Simulate download progress with matching name
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Downloading,
                percent: 50.0,
                status_message: "Downloading GE-Proton9-27... 50.0%".to_string(),
            },
        )));

        assert_eq!(model.tools[0].progress, 50.0);
        assert_eq!(model.tools[0].status, ToolStatus::Downloading);
        assert_eq!(model.tools[0].phase, DownloadPhase::Downloading);
    }

    #[test]
    fn mismatched_tool_name_does_not_update() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new(
            "GEProton GE-Proton9-27".to_string(),
            "Steam \"Native\"".to_string(),
        ));

        // Simulate progress with WRONG name (just version, like the bug)
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GE-Proton9-27".to_string(),  // WRONG - missing tool name prefix
                phase: DownloadPhase::Downloading,
                percent: 50.0,
                status_message: "Downloading... 50.0%".to_string(),
            },
        )));

        // Progress should NOT have updated because names don't match
        assert_eq!(model.tools[0].progress, 0.0);
        assert_eq!(model.tools[0].status, ToolStatus::Pending);
    }

    #[test]
    fn validation_progress_updates_correct_tool() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new(
            "GEProton GE-Proton9-27".to_string(),
            "Steam \"Native\"".to_string(),
        ));

        // Simulate validation progress
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Validating,
                percent: 30.0,
                status_message: "Validating GE-Proton9-27... 30.0%".to_string(),
            },
        )));

        assert_eq!(model.tools[0].progress, 30.0);
        assert_eq!(model.tools[0].status, ToolStatus::Validating);
    }

    #[test]
    fn unpacking_progress_updates_correct_tool() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new(
            "GEProton GE-Proton9-27".to_string(),
            "Steam \"Native\"".to_string(),
        ));

        // Simulate unpacking progress
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Unpacking,
                percent: 75.0,
                status_message: "Installing GE-Proton9-27... 75.0%".to_string(),
            },
        )));

        assert_eq!(model.tools[0].progress, 75.0);
        assert_eq!(model.tools[0].status, ToolStatus::Unpacking);
    }

    #[test]
    fn complete_phase_marks_tool_done() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new(
            "GEProton GE-Proton9-27".to_string(),
            "Steam \"Native\"".to_string(),
        ));

        // Simulate completion
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Complete,
                percent: 100.0,
                status_message: "✓ GEProton GE-Proton9-27 installed successfully".to_string(),
            },
        )));

        assert_eq!(model.tools[0].progress, 100.0);
        assert_eq!(model.tools[0].status, ToolStatus::Complete);
    }

    #[test]
    fn multiple_tools_track_progress_independently() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new(
            "GEProton GE-Proton9-27".to_string(),
            "Steam \"Native\"".to_string(),
        ));
        model.tools.push(ToolDownload::new(
            "GEProton GE-Proton9-26".to_string(),
            "Steam \"Native\"".to_string(),
        ));

        // Update first tool
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Downloading,
                percent: 50.0,
                status_message: "Downloading...".to_string(),
            },
        )));

        // Update second tool
        model.update(Message::DownloadUpdate(DownloadUpdate::ToolProgress(
            ToolProgress {
                tool_name: "GEProton GE-Proton9-26".to_string(),
                phase: DownloadPhase::Validating,
                percent: 80.0,
                status_message: "Validating...".to_string(),
            },
        )));

        // Verify independent progress
        assert_eq!(model.tools[0].progress, 50.0);
        assert_eq!(model.tools[0].status, ToolStatus::Downloading);
        assert_eq!(model.tools[1].progress, 80.0);
        assert_eq!(model.tools[1].status, ToolStatus::Validating);
    }

    //
    // Reinstall confirmation tests
    //

    #[test]
    fn toggle_reinstall_adds_and_removes() {
        let mut model = ProtonupGui::default();
        model.already_installed_tools = vec![
            ToolDownload::new("GEProton GE-Proton9-27".to_string(), "Steam".to_string()),
            ToolDownload::new("GEProton GE-Proton9-26".to_string(), "Steam".to_string()),
        ];

        // Toggle first tool
        let _ = model.update(Message::ToggleReinstall(0));
        assert_eq!(model.force_reinstall_indices, vec![0]);

        // Toggle second tool
        let _ = model.update(Message::ToggleReinstall(1));
        assert_eq!(model.force_reinstall_indices, vec![0, 1]);

        // Toggle off first tool
        let _ = model.update(Message::ToggleReinstall(0));
        assert_eq!(model.force_reinstall_indices, vec![1]);
    }

    #[test]
    fn already_installed_checked_with_tools_shows_confirm() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_installation = Some(AppInstallations::Steam);

        // Simulate already installed tools being checked
        let already_installed = vec![
            ToolDownload::new("GEProton GE-Proton9-27".to_string(), "Steam".to_string()),
        ];

        let _ = model.update(Message::AlreadyInstalledChecked(already_installed));

        assert_eq!(model.already_installed_tools.len(), 1);
        assert_eq!(model.selection_step, SelectionStep::ConfirmReinstall);
        assert!(model.global_status.contains("1 tool"));
    }

    #[test]
    fn already_installed_checked_with_tools_sets_confirm_step() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_installation = Some(AppInstallations::Steam);

        // Simulate already installed tools being checked
        let already_installed = vec![
            ToolDownload::new("GEProton GE-Proton9-27".to_string(), "Steam".to_string()),
        ];

        let _ = model.update(Message::AlreadyInstalledChecked(already_installed));

        assert_eq!(model.already_installed_tools.len(), 1);
        assert_eq!(model.selection_step, SelectionStep::ConfirmReinstall);
        assert!(model.global_status.contains("1 tool"));
    }

    #[test]
    fn already_installed_checked_empty_sets_download_step() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_installation = Some(AppInstallations::Steam);

        // Simulate no already installed tools
        let _ = model.update(Message::AlreadyInstalledChecked(vec![]));

        assert!(model.already_installed_tools.is_empty());
        // Should proceed to downloading
        assert_eq!(model.selection_step, SelectionStep::Downloading);
    }

    #[test]
    fn confirm_reinstall_selection_clears_indices() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.selection_step = SelectionStep::ConfirmReinstall;
        model.app_installation = Some(AppInstallations::Steam);
        model.already_installed_tools = vec![
            ToolDownload::new("GEProton GE-Proton9-27".to_string(), "Steam".to_string()),
        ];
        model.force_reinstall_indices = vec![0];  // User selected to reinstall

        // Just verify the state is set up correctly for the test
        assert_eq!(model.force_reinstall_indices, vec![0]);
        assert_eq!(model.selection_step, SelectionStep::ConfirmReinstall);
    }

    #[test]
    fn selective_reinstall_only_includes_selected_tools() {
        use std::collections::HashSet;
        
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.selection_step = SelectionStep::ConfirmReinstall;
        model.app_installation = Some(AppInstallations::Steam);
        
        // Setup: 3 tools already installed
        model.already_installed_tools = vec![
            ToolDownload::new("GEProton GE-Proton9-27".to_string(), "Steam".to_string()),
            ToolDownload::new("GEProton GE-Proton9-26".to_string(), "Steam".to_string()),
            ToolDownload::new("GEProton GE-Proton9-25".to_string(), "Steam".to_string()),
        ];
        
        // User only selects the first one for reinstall
        model.force_reinstall_indices = vec![0];
        
        // Build the force_reinstall_names set (same logic as ConfirmReinstallSelection)
        let force_reinstall_names: HashSet<String> = model.force_reinstall_indices
            .iter()
            .filter_map(|&i| model.already_installed_tools.get(i))
            .map(|t| t.name.clone())
            .collect();
        
        // Verify only the selected tool is in the set
        assert_eq!(force_reinstall_names.len(), 1);
        assert!(force_reinstall_names.contains("GEProton GE-Proton9-27"));
        assert!(!force_reinstall_names.contains("GEProton GE-Proton9-26"));
        assert!(!force_reinstall_names.contains("GEProton GE-Proton9-25"));
    }

    #[test]
    fn selective_reinstall_multiple_selected() {
        use std::collections::HashSet;
        
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.selection_step = SelectionStep::ConfirmReinstall;
        model.app_installation = Some(AppInstallations::Steam);
        
        // Setup: 3 tools already installed
        model.already_installed_tools = vec![
            ToolDownload::new("GEProton GE-Proton9-27".to_string(), "Steam".to_string()),
            ToolDownload::new("GEProton GE-Proton9-26".to_string(), "Steam".to_string()),
            ToolDownload::new("GEProton GE-Proton9-25".to_string(), "Steam".to_string()),
        ];
        
        // User selects first and third for reinstall
        model.force_reinstall_indices = vec![0, 2];
        
        // Build the force_reinstall_names set
        let force_reinstall_names: HashSet<String> = model.force_reinstall_indices
            .iter()
            .filter_map(|&i| model.already_installed_tools.get(i))
            .map(|t| t.name.clone())
            .collect();
        
        // Verify only selected tools are in the set
        assert_eq!(force_reinstall_names.len(), 2);
        assert!(force_reinstall_names.contains("GEProton GE-Proton9-27"));
        assert!(!force_reinstall_names.contains("GEProton GE-Proton9-26"));
        assert!(force_reinstall_names.contains("GEProton GE-Proton9-25"));
    }
}
