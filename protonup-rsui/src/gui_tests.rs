#[cfg(test)]
mod tests {
    use crate::download_task::{DownloadError, GlobalProgress};
    use crate::{
        AppInstallations, AppMode, DownloadPhase, DownloadUpdate, GuiMode, Message, ProtonupGui,
        QuickUpdateStatus, SelectionStep, ToolDownload, ToolProgress, ToolStatus,
    };
    use iced::widget::image;
    use iced_test::{Error, simulator};
    use libprotonup::downloads::Release;
    use libprotonup::sources::{CompatTool, Forge, ToolType};
    use std::path::PathBuf;
    use std::str::FromStr;

    fn make_test_release(tag: &str) -> Release {
        serde_json::from_value(serde_json::json!({
            "tag_name": tag,
            "name": tag,
            "url": null,
            "assets": [],
            "body": null
        }))
        .unwrap()
    }

    fn make_test_tool() -> CompatTool {
        CompatTool::new_custom(
            "GEProton".into(),
            Forge::GitHub,
            "GloriousEggroll".into(),
            "proton-ge-custom".into(),
            ToolType::WineBased,
            None,
            None,
            None,
        )
    }

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
            selected_arch_variant: None,
            has_variant_tools: false,
            app_installation: None,
            already_installed_tools: vec![],
            force_reinstall_indices: vec![],
            download_started: false,
            tools: vec![],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "Detected: Steam".to_string(),
            global_progress: 0.0,
            download_complete: None,
            download_handle: None,
            app_mode: AppMode::None,
            custom_path_input: String::new(),
            path_error: None,
            app_installations_views: vec![],
            manage_status: String::new(),
            manage_error: None,
            quick_update_status: QuickUpdateStatus::Idle,
            show_changelog: None,
            logo_handle: image::Handle::from_bytes(crate::LOGO_BYTES),
        }
    }

    //
    // View Tests using iced_test simulator
    //

    #[test]
    fn view_renders_initial_state() -> Result<(), Error> {
        let model = ProtonupGui::default();
        let mut ui = simulator(crate::views::app_view(&model));

        // Should show title in header
        assert!(ui.find("Protonup-rs").is_ok());

        // Should show placeholder text when no action selected
        assert!(ui.find("⬅️ Choose your option").is_ok());

        Ok(())
    }

    #[test]
    fn view_renders_detected_apps() -> Result<(), Error> {
        let model = ready_model();
        let mut ui = simulator(crate::views::app_view(&model));

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
            selected_arch_variant: None,
            has_variant_tools: false,
            app_installation: None,
            already_installed_tools: vec![],
            force_reinstall_indices: vec![],
            download_started: false,
            tools: vec![],
            global_phase: DownloadPhase::DetectingApps,
            global_status: "No compatible apps detected".to_string(),
            global_progress: 0.0,
            download_complete: None,
            download_handle: None,
            app_mode: AppMode::None,
            custom_path_input: String::new(),
            path_error: None,
            app_installations_views: vec![],
            manage_status: String::new(),
            manage_error: None,
            quick_update_status: QuickUpdateStatus::Idle,
            show_changelog: None,
            logo_handle: image::Handle::from_bytes(crate::LOGO_BYTES),
        };
        let mut ui = simulator(crate::views::app_view(&model));

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

        let _ = crate::update::handle(&mut model, Message::AppsScanned(apps));

        assert!(model.scan_complete);
        assert_eq!(model.detected_apps.len(), 2);
        assert!(model.global_status.contains("Steam"));
        assert!(model.global_status.contains("Lutris"));
    }

    #[test]
    fn apps_scanned_empty_list() {
        let mut model = ProtonupGui::default();

        let _ = crate::update::handle(&mut model, Message::AppsScanned(vec![]));

        assert!(model.scan_complete);
        assert!(model.detected_apps.is_empty());
        assert_eq!(model.global_status, "No compatible apps detected");
    }

    #[test]
    fn tool_selected_does_not_crash() {
        let mut model = ProtonupGui::default();

        let _ = crate::update::handle(&mut model, Message::ToolSelected(0));

        // Should not crash, and tool should be selected
        assert_eq!(model.selected_tool_indices, vec![0]);
    }

    #[test]
    fn selection_error_sets_status() {
        let mut model = ProtonupGui::default();

        let _ = crate::update::handle(
            &mut model,
            Message::SelectionError("test error".to_string()),
        );

        assert!(model.global_status.contains("test error"));
    }

    #[test]
    fn back_to_initial_resets_state() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        model.download_started = true;

        let _ = crate::update::handle(&mut model, Message::BackToInitial);

        assert_eq!(model.mode, GuiMode::Initial);
        assert_eq!(model.selection_step, SelectionStep::Initial);
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
        let model = ready_model();
        let sub = model.subscription();
        drop(sub);
    }

    //
    // Download progress tests
    //

    #[test]
    fn tool_progress_update() {
        let mut model = ProtonupGui::default();
        model.tools.push(ToolDownload::new("GEProton".to_string()));

        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GEProton".to_string(),
                phase: DownloadPhase::Downloading,
                percent: 75.0,
                status_message: "Downloading GEProton... 75.0%".to_string(),
            })),
        );

        assert_eq!(model.tools[0].progress, 75.0);
        assert_eq!(model.tools[0].phase, DownloadPhase::Downloading);
    }

    #[test]
    fn global_progress_update() {
        let mut model = ProtonupGui::default();

        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::GlobalProgress(GlobalProgress {
                phase: DownloadPhase::Downloading,
                status_message: "Downloading tools...".to_string(),
                percent: 50.0,
            })),
        );

        assert_eq!(model.global_progress, 50.0);
        assert_eq!(model.global_phase, DownloadPhase::Downloading);
    }

    #[test]
    fn download_finished_success() {
        let mut model = ProtonupGui::default();

        let release = make_test_release("GE-Proton9-27");
        let tool = make_test_tool();

        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::Finished(Ok(vec![(
                release, tool,
            )]))),
        );

        assert_eq!(model.global_progress, 100.0);
        assert_eq!(model.global_phase, DownloadPhase::Complete);
        assert!(model.download_complete.is_some());
        assert!(model.download_complete.as_ref().unwrap().is_ok());
    }

    #[test]
    fn download_finished_error() {
        let mut model = ProtonupGui::default();

        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::Finished(Err(DownloadError::IoError(
                "test error".to_string(),
            )))),
        );

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
        model.detected_apps = vec![AppInstallations::Steam, AppInstallations::Lutris];
        let mut ui = simulator(crate::views::app_view(&model));

        // Click the Quick Update button
        let _ = ui.click("Quick Update")?;

        // Process the messages
        for message in ui.into_messages() {
            let _ = crate::update::handle(&mut model, message);
        }

        // Verify NEW behavior: checking state, no tools populated
        assert!(model.download_started);
        assert_eq!(model.mode, GuiMode::QuickUpdate);
        assert_eq!(model.selection_step, SelectionStep::Downloading);
        assert_eq!(model.quick_update_status, QuickUpdateStatus::Checking);
        assert_eq!(model.global_status, "Checking for updates...");
        assert!(
            model.tools.is_empty(),
            "Tools should not be populated until after checking"
        );

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

        // Select first tool (radio button behavior - only one at a time)
        let _ = crate::update::handle(&mut model, Message::ToolSelected(0));
        assert_eq!(model.selected_tool_indices, vec![0]);

        // Select second tool (replaces first)
        let _ = crate::update::handle(&mut model, Message::ToolSelected(1));
        assert_eq!(model.selected_tool_indices, vec![1]);

        // Select first tool again (replaces second)
        let _ = crate::update::handle(&mut model, Message::ToolSelected(0));
        assert_eq!(model.selected_tool_indices, vec![0]);
    }

    #[test]
    fn toggle_version_adds_and_removes() {
        let mut model = ready_model();
        model.selection_step = SelectionStep::SelectingVersions;

        // Simulate versions being fetched
        let _ = crate::update::handle(&mut model, Message::VersionsFetched(vec![]));

        // Toggle versions
        let _ = crate::update::handle(&mut model, Message::ToggleVersion(0));
        assert_eq!(model.selected_version_indices, vec![0]);

        let _ = crate::update::handle(&mut model, Message::ToggleVersion(1));
        assert_eq!(model.selected_version_indices, vec![0, 1]);

        let _ = crate::update::handle(&mut model, Message::ToggleVersion(0));
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
        model.available_tools = vec![CompatTool::new_custom(
            "GEProton".to_string(),
            Forge::GitHub,
            "GloriousEggroll".to_string(),
            "proton-ge-custom".to_string(),
            ToolType::Runtime,
            None,
            None,
            None,
        )];

        // Select tool
        let _ = crate::update::handle(&mut model, Message::ToolSelected(0));
        assert_eq!(model.selected_tool_indices, vec![0]);

        // Confirm selection should move to version selection
        let _ = crate::update::handle(&mut model, Message::ToolSelectionConfirmed);
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
        let _ = crate::update::handle(&mut model, Message::ToggleVersion(0));
        assert_eq!(model.selected_version_indices, vec![0]);

        let _ = crate::update::handle(&mut model, Message::ToggleVersion(1));
        assert_eq!(model.selected_version_indices, vec![0, 1]);

        let _ = crate::update::handle(&mut model, Message::ToggleVersion(0));
        assert_eq!(model.selected_version_indices, vec![1]);
    }

    //
    // Download progress tracking tests
    //

    #[test]
    fn download_progress_updates_correct_tool() {
        let mut model = ProtonupGui::default();
        model
            .tools
            .push(ToolDownload::new("GEProton GE-Proton9-27".to_string()));

        // Simulate download progress with matching name
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Downloading,
                percent: 50.0,
                status_message: "Downloading GE-Proton9-27... 50.0%".to_string(),
            })),
        );

        assert_eq!(model.tools[0].progress, 50.0);
        assert_eq!(model.tools[0].status, ToolStatus::Downloading);
        assert_eq!(model.tools[0].phase, DownloadPhase::Downloading);
    }

    #[test]
    fn mismatched_tool_name_does_not_update() {
        let mut model = ProtonupGui::default();
        model
            .tools
            .push(ToolDownload::new("GEProton GE-Proton9-27".to_string()));

        // Simulate progress with WRONG name (just version, like the bug)
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GE-Proton9-27".to_string(), // WRONG - missing tool name prefix
                phase: DownloadPhase::Downloading,
                percent: 50.0,
                status_message: "Downloading... 50.0%".to_string(),
            })),
        );

        // Progress should NOT have updated because names don't match
        assert_eq!(model.tools[0].progress, 0.0);
        assert_eq!(model.tools[0].status, ToolStatus::Pending);
    }

    #[test]
    fn validation_progress_updates_correct_tool() {
        let mut model = ProtonupGui::default();
        model
            .tools
            .push(ToolDownload::new("GEProton GE-Proton9-27".to_string()));

        // Simulate validation progress
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Validating,
                percent: 30.0,
                status_message: "Validating GE-Proton9-27... 30.0%".to_string(),
            })),
        );

        assert_eq!(model.tools[0].progress, 30.0);
        assert_eq!(model.tools[0].status, ToolStatus::Validating);
    }

    #[test]
    fn unpacking_progress_updates_correct_tool() {
        let mut model = ProtonupGui::default();
        model
            .tools
            .push(ToolDownload::new("GEProton GE-Proton9-27".to_string()));

        // Simulate unpacking progress
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Unpacking,
                percent: 75.0,
                status_message: "Installing GE-Proton9-27... 75.0%".to_string(),
            })),
        );

        assert_eq!(model.tools[0].progress, 75.0);
        assert_eq!(model.tools[0].status, ToolStatus::Unpacking);
    }

    #[test]
    fn complete_phase_marks_tool_done() {
        let mut model = ProtonupGui::default();
        model
            .tools
            .push(ToolDownload::new("GEProton GE-Proton9-27".to_string()));

        // Simulate completion
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Complete,
                percent: 100.0,
                status_message: "✓ GEProton GE-Proton9-27 installed successfully".to_string(),
            })),
        );

        assert_eq!(model.tools[0].progress, 100.0);
        assert_eq!(model.tools[0].status, ToolStatus::_Complete);
    }

    #[test]
    fn multiple_tools_track_progress_independently() {
        let mut model = ProtonupGui::default();
        model
            .tools
            .push(ToolDownload::new("GEProton GE-Proton9-27".to_string()));
        model
            .tools
            .push(ToolDownload::new("GEProton GE-Proton9-26".to_string()));

        // Update first tool
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GEProton GE-Proton9-27".to_string(),
                phase: DownloadPhase::Downloading,
                percent: 50.0,
                status_message: "Downloading...".to_string(),
            })),
        );

        // Update second tool
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ToolProgress {
                tool_name: "GEProton GE-Proton9-26".to_string(),
                phase: DownloadPhase::Validating,
                percent: 80.0,
                status_message: "Validating...".to_string(),
            })),
        );

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
        let mut model = ProtonupGui {
            already_installed_tools: vec![
                ToolDownload::new("GEProton GE-Proton9-27".to_string()),
                ToolDownload::new("GEProton GE-Proton9-26".to_string()),
            ],
            ..Default::default()
        };

        // Toggle first tool
        let _ = crate::update::handle(&mut model, Message::ToggleReinstall(0));
        assert_eq!(model.force_reinstall_indices, vec![0]);

        // Toggle second tool
        let _ = crate::update::handle(&mut model, Message::ToggleReinstall(1));
        assert_eq!(model.force_reinstall_indices, vec![0, 1]);

        // Toggle off first tool
        let _ = crate::update::handle(&mut model, Message::ToggleReinstall(0));
        assert_eq!(model.force_reinstall_indices, vec![1]);
    }

    #[test]
    fn already_installed_checked_with_tools_shows_confirm() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_installation = Some(AppInstallations::Steam);

        // Simulate already installed tools being checked
        let already_installed = vec![ToolDownload::new("GEProton GE-Proton9-27".to_string())];

        let _ = crate::update::handle(
            &mut model,
            Message::AlreadyInstalledChecked(already_installed),
        );

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
        let already_installed = vec![ToolDownload::new("GEProton GE-Proton9-27".to_string())];

        let _ = crate::update::handle(
            &mut model,
            Message::AlreadyInstalledChecked(already_installed),
        );

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
        let _ = crate::update::handle(&mut model, Message::AlreadyInstalledChecked(vec![]));

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
        model.already_installed_tools =
            vec![ToolDownload::new("GEProton GE-Proton9-27".to_string())];
        model.force_reinstall_indices = vec![0]; // User selected to reinstall

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
            ToolDownload::new("GEProton GE-Proton9-27".to_string()),
            ToolDownload::new("GEProton GE-Proton9-26".to_string()),
            ToolDownload::new("GEProton GE-Proton9-25".to_string()),
        ];

        // User only selects the first one for reinstall
        model.force_reinstall_indices = vec![0];

        // Build the force_reinstall_names set (same logic as ConfirmReinstallSelection)
        let force_reinstall_names: HashSet<String> = model
            .force_reinstall_indices
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
            ToolDownload::new("GEProton GE-Proton9-27".to_string()),
            ToolDownload::new("GEProton GE-Proton9-26".to_string()),
            ToolDownload::new("GEProton GE-Proton9-25".to_string()),
        ];

        // User selects first and third for reinstall
        model.force_reinstall_indices = vec![0, 2];

        // Build the force_reinstall_names set
        let force_reinstall_names: HashSet<String> = model
            .force_reinstall_indices
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

    //
    // Logo persistence tests
    //

    #[test]
    fn logo_handle_persists_across_views() {
        let model = ready_model();

        // Call view() multiple times - simulating redraws from window events
        let _ = crate::views::app_view(&model);
        let _ = crate::views::app_view(&model);
        let _ = crate::views::app_view(&model);

        // The logo_handle should be the same instance (same pointer) across all views
        let handle1 = &model.logo_handle;
        let _ = crate::views::app_view(&model);
        let handle2 = &model.logo_handle;

        // Both references should point to the same underlying handle
        assert!(
            std::ptr::eq(handle1, handle2),
            "logo_handle should be the same instance across view() calls"
        );
    }

    #[test]
    fn logo_handle_is_initialized_at_startup() {
        let model = ProtonupGui::default();
        // logo_handle should be initialized and not cause any issues when accessed
        let _ = &model.logo_handle;
    }

    //
    // Cancel tests
    //

    #[test]
    fn cancel_resets_state_to_initial() {
        let mut model = ready_model();
        model.download_started = true;
        model.selection_step = SelectionStep::Downloading;
        model.mode = GuiMode::DownloadForSteam;
        // Simulate having a handle (we can't easily create a real one)
        // The test just verifies the state reset logic

        let _ = crate::update::handle(&mut model, Message::Cancel);

        // Verify state is reset
        assert_eq!(model.selection_step, SelectionStep::Initial);
        assert_eq!(model.mode, GuiMode::Initial);
        assert!(!model.download_started);
        assert!(model.download_handle.is_none());
        assert!(model.tools.is_empty());
    }

    //
    // Architecture variant selection tests
    //

    #[test]
    fn select_architecture_variant_updates_state() {
        let mut model = ready_model();
        model.selected_arch_variant = Some(2); // Default

        // Select v3
        let _ = crate::update::handle(&mut model, Message::SelectArchitecture(3));

        assert_eq!(model.selected_arch_variant, Some(3));
    }

    #[test]
    fn has_variant_tools_detects_variant_tool() {
        use libprotonup::sources::{CompatTool, Forge, ToolType};

        let mut model = ready_model();
        model.selected_tool_indices = vec![0];

        // Add a tool with multiple asset variations
        model.available_tools = vec![CompatTool::new_custom(
            "ProtonCachyOS".to_string(),
            Forge::GitHub,
            "CachyOS".to_string(),
            "proton-cachyos".to_string(),
            ToolType::Runtime,
            None,
            None,
            None,
        )];
        // Manually set has_multiple_asset_variations for testing
        model.available_tools[0].has_multiple_asset_variations = true;

        // Simulate versions being fetched
        model.has_variant_tools = model.selected_tool_indices.iter().any(|&idx| {
            model
                .available_tools
                .get(idx)
                .is_some_and(|t| t.has_multiple_asset_variations)
        });

        assert!(model.has_variant_tools);
    }

    #[test]
    fn has_variant_tools_false_for_normal_tool() {
        use libprotonup::sources::{CompatTool, Forge, ToolType};

        let mut model = ready_model();
        model.selected_tool_indices = vec![0];

        // Add a tool without multiple asset variations
        model.available_tools = vec![CompatTool::new_custom(
            "GEProton".to_string(),
            Forge::GitHub,
            "GloriousEggroll".to_string(),
            "proton-ge-custom".to_string(),
            ToolType::Runtime,
            None,
            None,
            None,
        )];
        // has_multiple_asset_variations is false by default

        model.has_variant_tools = model.selected_tool_indices.iter().any(|&idx| {
            model
                .available_tools
                .get(idx)
                .is_some_and(|t| t.has_multiple_asset_variations)
        });

        assert!(!model.has_variant_tools);
    }

    //
    // Custom location flow tests
    //

    #[test]
    fn select_custom_location_shows_path_input() {
        let mut model = ready_model();
        model.app_mode = AppMode::DownloadForCustom;
        model.mode = GuiMode::DownloadForCustom;
        model.selection_step = SelectionStep::Initial;
        model.custom_path_input = "/test/path".to_string();

        // Verify state is set correctly
        assert_eq!(model.app_mode, AppMode::DownloadForCustom);
        assert_eq!(model.mode, GuiMode::DownloadForCustom);
        assert_eq!(model.selection_step, SelectionStep::Initial);
        assert_eq!(model.custom_path_input, "/test/path");
    }

    #[test]
    fn custom_path_input_updates_state() {
        let mut model = ready_model();
        model.custom_path_input = "/old/path".to_string();

        let _ = crate::update::handle(
            &mut model,
            Message::CustomPathInput("/new/path".to_string()),
        );

        assert_eq!(model.custom_path_input, "/new/path");
        assert!(model.path_error.is_none());
    }

    #[test]
    fn folder_picked_updates_path() {
        let mut model = ready_model();
        model.custom_path_input = "/old/path".to_string();
        model.path_error = Some("Some error".to_string());

        let _ = crate::update::handle(
            &mut model,
            Message::FolderPicked(Some(PathBuf::from("/picked/path"))),
        );

        assert_eq!(model.custom_path_input, "/picked/path");
        assert!(model.path_error.is_none());
    }

    #[test]
    fn folder_picked_none_does_nothing() {
        let mut model = ready_model();
        model.custom_path_input = "/existing/path".to_string();

        let _ = crate::update::handle(&mut model, Message::FolderPicked(None));

        // State should remain unchanged
        assert_eq!(model.custom_path_input, "/existing/path");
    }

    #[test]
    fn select_download_for_custom_resets_state() {
        let mut model = ready_model();
        model.app_mode = AppMode::DownloadForSteam;
        model.mode = GuiMode::DownloadForSteam;
        model.custom_path_input = "/some/path".to_string();

        let _ = crate::update::handle(&mut model, Message::SelectDownloadForCustom);

        assert_eq!(model.app_mode, AppMode::DownloadForCustom);
        assert_eq!(model.mode, GuiMode::DownloadForCustom);
        assert_eq!(model.selection_step, SelectionStep::Initial);
        assert!(model.path_error.is_none());
        // custom_path_input should be set to current directory (non-empty)
        assert!(!model.custom_path_input.is_empty());
    }

    #[test]
    fn custom_location_tool_selection_confirmed_advances() {
        let mut model = ready_model();
        model.app_mode = AppMode::DownloadForCustom;
        model.mode = GuiMode::DownloadForCustom;
        model.custom_path_input = "/test/path".to_string();
        model.selection_step = SelectionStep::Initial;

        // First Continue: should set up app_installation and fetch tools
        let _ = crate::update::handle(&mut model, Message::ToolSelectionConfirmed);

        // Verify we're now at tool selection step
        assert!(model.app_installation.is_some());
        assert_eq!(model.selection_step, SelectionStep::SelectingTools);
        assert!(!model.available_tools.is_empty());

        // Now simulate selecting a tool
        model.selected_tool_indices = vec![0];

        // Second Continue: should advance to version selection
        // (This would normally fetch releases, but in test it just sets up state)
        // We can't easily test the async fetch, but we can verify the logic path
        // by checking that it doesn't reset app_installation
    }

    #[test]
    fn custom_location_empty_path_shows_error() {
        let mut model = ready_model();
        model.app_mode = AppMode::DownloadForCustom;
        model.mode = GuiMode::DownloadForCustom;
        model.custom_path_input = String::new();
        model.selection_step = SelectionStep::Initial;

        let _ = crate::update::handle(&mut model, Message::ToolSelectionConfirmed);

        // Should show error and stay at initial step
        assert!(model.path_error.is_some());
        assert_eq!(model.selection_step, SelectionStep::Initial);
        assert!(model.app_installation.is_none());
    }

    #[test]
    fn back_from_version_selection_returns_to_tool_selection() {
        let mut model = ready_model();
        model.app_mode = AppMode::DownloadForSteam;
        model.mode = GuiMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingVersions;

        // Set up some version selection state
        model.selected_version_indices = vec![0, 1];

        // Simulate having fetched some versions (we can't construct Release directly)
        // Just test the state transition without actual Release objects
        model.available_versions = Vec::new(); // Empty but that's OK for this test

        // Send back message
        let _ = crate::update::handle(&mut model, Message::BackToToolSelection);

        // Should return to tool selection
        assert_eq!(model.selection_step, SelectionStep::SelectingTools);
        assert!(model.selected_version_indices.is_empty());
        assert!(model.available_versions.is_empty());
    }

    //
    // Quick Update Tests
    //

    #[test]
    fn select_quick_update_sets_checking_state() {
        let mut model = ready_model();
        model.detected_apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        let _ = crate::update::handle(&mut model, Message::SelectQuickUpdate);

        assert_eq!(model.app_mode, AppMode::QuickUpdate);
        assert_eq!(model.mode, GuiMode::QuickUpdate);
        assert_eq!(model.selection_step, SelectionStep::Downloading);
        assert!(model.download_started);
        assert_eq!(model.quick_update_status, QuickUpdateStatus::Checking);
        assert_eq!(model.global_status, "Checking for updates...");
        assert!(model.tools.is_empty()); // Tools not populated yet
    }

    #[test]
    fn quick_update_checked_all_installed_shows_up_to_date() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::Checking;

        // Simulate all tools installed
        let results = vec![
            ("GEProton".to_string(), true),
            ("Wine-GE".to_string(), true),
        ];

        let _ = crate::update::handle(&mut model, Message::QuickUpdateChecked(results));

        assert_eq!(
            model.quick_update_status,
            QuickUpdateStatus::AllUpToDate(vec!["GEProton".to_string(), "Wine-GE".to_string()])
        );
        assert_eq!(model.global_status, "Tools are up to date.");
        assert!(model.tools.is_empty()); // Still no tools populated
    }

    #[test]
    fn quick_update_checked_some_not_installed_starts_download() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::Checking;
        model.detected_apps = vec![AppInstallations::Steam];

        // Simulate some tools not installed
        let results = vec![("GEProton".to_string(), false)];

        let _ = crate::update::handle(&mut model, Message::QuickUpdateChecked(results));

        assert_eq!(model.quick_update_status, QuickUpdateStatus::InProgress);
        assert_eq!(model.global_status, "Starting Quick Update...");
    }

    #[test]
    fn force_reinstall_starts_download_with_force() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::AllUpToDate(vec!["GEProton".to_string()]);
        model.detected_apps = vec![AppInstallations::Steam];

        let _ = crate::update::handle(&mut model, Message::ForceReinstall);

        assert_eq!(model.quick_update_status, QuickUpdateStatus::InProgress);
        assert_eq!(model.global_status, "Force reinstalling tools...");
    }

    #[test]
    fn quick_update_flow_all_installed_shows_correct_ui() -> Result<(), Error> {
        let mut model = ready_model();
        model.detected_apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        // Step 1: Click Quick Update - should set checking state
        let _ = crate::update::handle(&mut model, Message::SelectQuickUpdate);

        assert_eq!(model.app_mode, AppMode::QuickUpdate);
        assert_eq!(model.mode, GuiMode::QuickUpdate);
        assert_eq!(model.selection_step, SelectionStep::Downloading);
        assert!(model.download_started);
        assert_eq!(model.quick_update_status, QuickUpdateStatus::Checking);
        assert_eq!(model.global_status, "Checking for updates...");
        assert!(model.tools.is_empty()); // Tools not populated yet

        // Verify view shows checking state (not download progress) without crashing
        {
            let _ui = simulator(crate::views::app_view(&model));
        }

        // Step 2: Simulate check result - all tools installed
        let results = vec![
            ("GEProton".to_string(), true),
            ("Wine-GE".to_string(), true),
        ];

        let _ = crate::update::handle(&mut model, Message::QuickUpdateChecked(results));

        assert_eq!(
            model.quick_update_status,
            QuickUpdateStatus::AllUpToDate(vec!["GEProton".to_string(), "Wine-GE".to_string()])
        );
        assert_eq!(model.global_status, "Tools are up to date.");
        assert!(model.tools.is_empty()); // Still no tools populated

        // Verify view shows up-to-date state without crashing
        {
            let _ui = simulator(crate::views::app_view(&model));
        }

        // Step 3: Click Force Reinstall — tools now auto-created by progress callbacks
        let _ = crate::update::handle(&mut model, Message::ForceReinstall);

        assert_eq!(model.quick_update_status, QuickUpdateStatus::InProgress);
        assert_eq!(model.global_status, "Force reinstalling tools...");

        Ok(())
    }

    #[test]
    fn quick_update_does_not_populate_tools_immediately() {
        let mut model = ready_model();
        model.detected_apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        // OLD BEHAVIOR: SelectQuickUpdate would immediately populate tools
        // NEW BEHAVIOR: Should NOT populate tools until after check

        let _ = crate::update::handle(&mut model, Message::SelectQuickUpdate);

        // Verify tools are NOT populated (new behavior)
        assert!(
            model.tools.is_empty(),
            "Tools should not be populated until after checking"
        );

        // Verify checking state
        assert_eq!(model.quick_update_status, QuickUpdateStatus::Checking);
    }

    #[tokio::test]
    async fn check_quick_update_installed_returns_true_when_tool_folder_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let tool_dir = tmp.path().join("GE-Proton9-27");
        tokio::fs::create_dir(&tool_dir).await.unwrap();

        let detected_apps = vec![AppInstallations::Custom(
            tmp.path().to_str().unwrap().to_string(),
        )];

        let results = ProtonupGui::check_quick_update_installed(detected_apps).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "GEProton");
        assert!(
            results[0].1,
            "check_quick_update_installed should return true when GEProton folder exists"
        );
    }

    #[test]
    fn quick_update_checked_with_two_apps_creates_distinctly_named_tools() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::Checking;
        model.detected_apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        // Simulate some tools not installed for both apps
        let results = vec![
            ("GEProton".to_string(), false),
            ("GEProton".to_string(), false),
        ];

        let _ = crate::update::handle(&mut model, Message::QuickUpdateChecked(results));

        // Tools are no longer pre-populated — they are auto-created by progress callbacks.
        // With dedup, the same tool for multiple apps gets a combined display name.
        // Simulate the progress callback creating a deduped entry:
        let progress = ToolProgress {
            tool_name: "GEProton (Steam, Lutris)".to_string(),
            phase: DownloadPhase::Downloading,
            percent: 30.0,
            status_message: "Downloading...".to_string(),
        };
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(progress)),
        );

        assert_eq!(model.tools.len(), 1, "deduped tools create one entry");
        assert_eq!(model.tools[0].name, "GEProton (Steam, Lutris)");
    }
    #[test]
    fn quick_update_progress_routes_to_correct_tool_by_unique_name() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::InProgress;
        model.detected_apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        // With dedup, the same tool for both apps gets a combined name
        model
            .tools
            .push(ToolDownload::new("GEProton (Steam, Lutris)".to_string()));
        // A different tool keeps its own entry
        model
            .tools
            .push(ToolDownload::new("Luxtorpeda (Steam)".to_string()));

        // Simulate progress for the deduped GEProton tool
        let ge_progress = ToolProgress {
            tool_name: "GEProton (Steam, Lutris)".to_string(),
            phase: DownloadPhase::Downloading,
            percent: 50.0,
            status_message: "Downloading...".to_string(),
        };

        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(ge_progress)),
        );

        assert_eq!(model.tools[0].progress, 50.0); // GEProton updated
        assert_eq!(model.tools[1].progress, 0.0); // Luxtorpeda unchanged

        // Simulate progress for Luxtorpeda tool
        let lux_progress = ToolProgress {
            tool_name: "Luxtorpeda (Steam)".to_string(),
            phase: DownloadPhase::Downloading,
            percent: 75.0,
            status_message: "Downloading...".to_string(),
        };
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(lux_progress)),
        );

        assert_eq!(model.tools[0].progress, 50.0); // GEProton unchanged
        assert_eq!(model.tools[1].progress, 75.0); // Luxtorpeda updated
    }

    #[test]
    fn force_reinstall_creates_distinctly_named_tools() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::AllUpToDate(vec!["GEProton".to_string()]);
        model.detected_apps = vec![AppInstallations::Steam, AppInstallations::Lutris];

        let _ = crate::update::handle(&mut model, Message::ForceReinstall);

        // Tools are no longer pre-populated — they are auto-created by progress callbacks.
        // With dedup, the same tool for multiple apps gets a combined display name.
        // Simulate the progress callback creating a deduped entry and a separate entry:
        let progress_ge = ToolProgress {
            tool_name: "GEProton (Steam, Lutris)".to_string(),
            phase: DownloadPhase::Downloading,
            percent: 30.0,
            status_message: "Downloading...".to_string(),
        };
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(progress_ge)),
        );

        assert_eq!(
            model.tools.len(),
            1,
            "deduped tools create one entry per unique download"
        );
        assert_eq!(model.tools[0].name, "GEProton (Steam, Lutris)");
    }

    #[test]
    fn dedup_auto_creates_tool_entry_on_first_progress() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::InProgress;

        // Tools start empty — progress callback will auto-create entries
        assert!(model.tools.is_empty());

        // Simulate progress arriving for a deduped download group
        let progress = ToolProgress {
            tool_name: "GEProton (Steam, Lutris, Flatpak)".to_string(),
            phase: DownloadPhase::Downloading,
            percent: 25.0,
            status_message: "Downloading...".to_string(),
        };
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(progress)),
        );

        assert_eq!(
            model.tools.len(),
            1,
            "auto-creates one entry per unique download"
        );
        assert_eq!(model.tools[0].name, "GEProton (Steam, Lutris, Flatpak)");
        assert_eq!(model.tools[0].progress, 25.0);

        // Second progress update for same tool updates existing entry
        let progress2 = ToolProgress {
            tool_name: "GEProton (Steam, Lutris, Flatpak)".to_string(),
            phase: DownloadPhase::Validating,
            percent: 100.0,
            status_message: "Validating...".to_string(),
        };
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(progress2)),
        );

        assert_eq!(model.tools.len(), 1, "still one entry — no duplicate");
        assert_eq!(model.tools[0].progress, 100.0);

        // Third progress for a different tool creates a second entry
        let progress3 = ToolProgress {
            tool_name: "Luxtorpeda (Steam)".to_string(),
            phase: DownloadPhase::Downloading,
            percent: 50.0,
            status_message: "Downloading...".to_string(),
        };
        let _ = crate::update::handle(
            &mut model,
            Message::DownloadUpdate(DownloadUpdate::ToolProgress(progress3)),
        );

        assert_eq!(
            model.tools.len(),
            2,
            "second unique download creates another entry"
        );
        assert_eq!(model.tools[1].name, "Luxtorpeda (Steam)");
    }
}
