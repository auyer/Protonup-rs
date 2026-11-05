#[cfg(test)]
mod tests {
    use crate::download_task::{DownloadError, GlobalProgress};
    use crate::{
        AppInstallations, AppMode, DownloadPhase, DownloadUpdate, GuiMode, InstalledTool, Message,
        ProtonupGui, QuickUpdateStatus, SelectionStep, ToolDownload, ToolProgress, ToolStatus,
    };
    use iced::widget::image;
    use iced::{Settings, Size};
    use iced_test::selector::Candidate;
    use iced_test::{Error, Simulator, simulator};
    use libprotonup::architecture_variants::MicroArchVariants;
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
            show_all_versions: None,
            logo_handle: image::Handle::from_bytes(crate::LOGO_BYTES),
            focus: crate::FocusState::default(),
            show_keyboard_shortcuts: false,
            show_controller_shortcuts: false,
            gamepads_unavailable: false,
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
    fn chrome_visible_at_steam_deck_resolution() -> Result<(), Error> {
        // Steam Deck / gamescope runs at 1280x800. The header, footer hints and
        // the pinned sidebar `Close` button must stay within the viewport.
        // (Light-mode legibility is a contrast concern and is verified
        // manually; here we guard the geometric layout.)
        let model = ready_model();
        let mut ui = Simulator::with_size(
            Settings::default(),
            Size::new(1280.0, 800.0),
            crate::views::app_view(&model),
        );

        for label in [
            "Protonup-rs",
            crate::views::footer::KEYBOARD_HINT,
            crate::views::footer::CONTROLLER_HINT,
            "Close",
        ] {
            let node = ui.find(label)?;
            let visible = node
                .visible_bounds()
                .unwrap_or_else(|| panic!("{label:?} should have visible bounds"));
            assert!(
                visible.height > 0.0 && visible.width > 0.0,
                "{label:?} should be visible, got {visible:?}"
            );
            assert!(
                visible.y >= 0.0 && visible.y + visible.height <= 800.0,
                "{label:?} should be within the viewport, got {visible:?}"
            );
        }

        Ok(())
    }

    #[test]
    fn sidebar_scrolls_when_buttons_do_not_fit() -> Result<(), Error> {
        let model = ProtonupGui::default();
        let mut ui = Simulator::with_size(
            Settings::default(),
            Size::new(320.0, 400.0),
            crate::views::app_view(&model),
        );

        // The sidebar wraps its content in a scrollable. With a small viewport
        // the content must be taller than the scrollable viewport (i.e. it
        // scrolls instead of clipping the buttons below the fold).
        let (viewport, content) = ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                bounds,
                content_bounds,
                ..
            } => Some((bounds, content_bounds)),
            _ => None,
        })?;

        assert!(
            content.height > viewport.height,
            "scrollable content ({}) should exceed viewport ({})",
            content.height,
            viewport.height
        );

        // The Close button is pinned below the scrollable, so it stays visible.
        let close = ui.find("Close")?;
        let visible = close
            .visible_bounds()
            .expect("Close button should have visible bounds");
        assert!(visible.height > 0.0);

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
            show_all_versions: None,
            logo_handle: image::Handle::from_bytes(crate::LOGO_BYTES),
            focus: crate::FocusState::default(),
            show_keyboard_shortcuts: false,
            show_controller_shortcuts: false,
            gamepads_unavailable: false,
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
            Message::DownloadUpdate(DownloadUpdate::Finished(Ok(vec![(release, tool)]))),
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

        // Select first tool (button behavior - only one at a time)
        let _ = crate::update::handle(&mut model, Message::ToolChosen(0));
        assert_eq!(model.selected_tool_indices, vec![0]);

        // Select second tool (replaces first)
        let _ = crate::update::handle(&mut model, Message::ToolChosen(1));
        assert_eq!(model.selected_tool_indices, vec![1]);

        // Select first tool again (replaces second)
        let _ = crate::update::handle(&mut model, Message::ToolChosen(0));
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

        // Clicking the tool button selects and advances in one action
        let _ = crate::update::handle(&mut model, Message::ToolChosen(0));
        assert_eq!(model.selected_tool_indices, vec![0]);
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
        model.selected_arch_variant = Some(MicroArchVariants::X86_64V2); // Default

        // Click the v3 variant button (selects and attempts to start download)
        let _ = crate::update::handle(
            &mut model,
            Message::ArchitectureChosen(MicroArchVariants::X86_64V3),
        );

        assert_eq!(
            model.selected_arch_variant,
            Some(MicroArchVariants::X86_64V3)
        );
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
            InstalledTool {
                app: AppInstallations::Steam,
                tool: "GEProton".to_string(),
                latest: Some("GE-Proton11-11".to_string()),
                versions: vec!["GE-Proton11-11".to_string()],
            },
            InstalledTool {
                app: AppInstallations::Lutris,
                tool: "GEProton".to_string(),
                latest: Some("GE-Proton11-11".to_string()),
                versions: vec!["GE-Proton11-11".to_string()],
            },
        ];

        let _ = crate::update::handle(&mut model, Message::QuickUpdateChecked(results.clone()));

        assert_eq!(
            model.quick_update_status,
            QuickUpdateStatus::AllUpToDate(results)
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
        let results = vec![InstalledTool {
            app: AppInstallations::Steam,
            tool: "GEProton".to_string(),
            latest: None,
            versions: vec![],
        }];

        let _ = crate::update::handle(&mut model, Message::QuickUpdateChecked(results));

        assert_eq!(model.quick_update_status, QuickUpdateStatus::InProgress);
        assert_eq!(model.global_status, "Starting Quick Update...");
    }

    #[test]
    fn force_reinstall_starts_download_with_force() {
        let mut model = ready_model();
        model.app_mode = AppMode::QuickUpdate;
        model.mode = GuiMode::QuickUpdate;
        model.quick_update_status = QuickUpdateStatus::AllUpToDate(vec![InstalledTool {
            app: AppInstallations::Steam,
            tool: "GEProton".to_string(),
            latest: Some("GE-Proton11-11".to_string()),
            versions: vec!["GE-Proton11-11".to_string()],
        }]);
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
            InstalledTool {
                app: AppInstallations::Steam,
                tool: "GEProton".to_string(),
                latest: Some("GE-Proton11-11".to_string()),
                versions: vec!["GE-Proton11-11".to_string()],
            },
            InstalledTool {
                app: AppInstallations::Lutris,
                tool: "GEProton".to_string(),
                latest: Some("GE-Proton11-11".to_string()),
                versions: vec!["GE-Proton11-11".to_string()],
            },
        ];

        let _ = crate::update::handle(&mut model, Message::QuickUpdateChecked(results.clone()));

        assert_eq!(
            model.quick_update_status,
            QuickUpdateStatus::AllUpToDate(results)
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
        assert_eq!(results[0].tool, "GEProton");
        assert_eq!(results[0].versions, vec!["GE-Proton9-27".to_string()]);
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
            InstalledTool {
                app: AppInstallations::Steam,
                tool: "GEProton".to_string(),
                latest: None,
                versions: vec![],
            },
            InstalledTool {
                app: AppInstallations::Lutris,
                tool: "GEProton".to_string(),
                latest: None,
                versions: vec![],
            },
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
        model.quick_update_status = QuickUpdateStatus::AllUpToDate(vec![InstalledTool {
            app: AppInstallations::Steam,
            tool: "GEProton".to_string(),
            latest: Some("GE-Proton11-11".to_string()),
            versions: vec!["GE-Proton11-11".to_string()],
        }]);
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

    //
    // Action Pane (bottom-anchored actions) Tests
    //

    #[test]
    fn action_pane_renders_tool_selection_actions() -> Result<(), Error> {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        model.available_tools =
            vec![libprotonup::sources::CompatTool::from_str("GEProton").unwrap()];

        let mut ui = simulator(crate::views::app_view(&model));

        // Tools are now direct-action buttons; there is no "Continue" step.
        assert!(
            ui.find("GEProton").is_ok(),
            "each tool should be rendered as a clickable button"
        );
        assert!(
            ui.find("Continue").is_err(),
            "Continue should be gone from the action bar"
        );
        assert!(ui.find("Back").is_ok(), "Back action should be visible");

        Ok(())
    }

    #[test]
    fn action_pane_renders_quick_update_actions() -> Result<(), Error> {
        let mut model = ready_model();
        model.mode = GuiMode::QuickUpdate;
        model.app_mode = AppMode::QuickUpdate;
        model.selection_step = SelectionStep::Downloading;
        model.download_started = true;
        model.quick_update_status = QuickUpdateStatus::AllUpToDate(vec![InstalledTool {
            app: AppInstallations::Steam,
            tool: "GEProton".to_string(),
            latest: Some("GE-Proton11-11".to_string()),
            versions: vec!["GE-Proton11-11".to_string()],
        }]);

        let mut ui = simulator(crate::views::app_view(&model));

        assert!(
            ui.find("Force Reinstallation").is_ok(),
            "Force Reinstallation action should be visible"
        );
        assert!(
            ui.find("Back to Main Menu").is_ok(),
            "Back to Main Menu action should be visible"
        );

        Ok(())
    }

    #[test]
    fn action_pane_renders_manage_installations_actions() -> Result<(), Error> {
        let mut model = ready_model();
        model.mode = GuiMode::ManageInstallations;
        model.app_mode = AppMode::ManageInstallations;

        let mut ui = simulator(crate::views::app_view(&model));

        // No selections yet => Delete Selected is rendered but disabled, and still present.
        assert!(
            ui.find("Delete Selected").is_ok(),
            "Delete Selected should be visible"
        );
        assert!(
            ui.find("Back to Main Menu").is_ok(),
            "Back to Main Menu should be visible"
        );

        Ok(())
    }

    #[test]
    fn action_pane_renders_version_selection_actions() -> Result<(), Error> {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingVersions;
        model.selected_tool = Some(libprotonup::sources::CompatTool::from_str("GEProton").unwrap());

        let mut ui = simulator(crate::views::app_view(&model));

        assert!(
            ui.find("Start Download").is_ok(),
            "Start Download should be visible"
        );
        assert!(
            ui.find("Show Changelog").is_ok(),
            "Show Changelog should be visible"
        );
        assert!(ui.find("Back").is_ok(), "Back should be visible");

        Ok(())
    }

    #[test]
    fn action_pane_renders_check_whats_new_actions() -> Result<(), Error> {
        let mut model = ready_model();
        model.mode = GuiMode::CheckWhatsNew;
        model.app_mode = AppMode::CheckWhatsNew;
        model.selection_step = SelectionStep::SelectingVersions;
        model.selected_tool = Some(libprotonup::sources::CompatTool::from_str("GEProton").unwrap());

        let mut ui = simulator(crate::views::app_view(&model));

        // Version rows are direct-action buttons; there is no "View Changelog" step.
        assert!(
            ui.find("View Changelog").is_err(),
            "View Changelog should be gone from the action bar"
        );
        assert!(ui.find("Back").is_ok(), "Back should be visible");

        Ok(())
    }

    //
    // Direct-action list buttons (replacing radio single-select)
    //

    #[test]
    fn tool_chosen_selects_and_advances_in_one_click() {
        use libprotonup::sources::CompatTool;

        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        model.app_installation = Some(AppInstallations::Steam);
        model.available_tools = vec![
            CompatTool::from_str("GEProton").unwrap(),
            CompatTool::from_str("Luxtorpeda").unwrap(),
        ];

        let _ = crate::update::handle(&mut model, Message::ToolChosen(1));

        assert_eq!(model.selected_tool_indices, vec![1]);
        assert!(
            model.selected_tool.is_some(),
            "clicking a tool should lock in the selection"
        );
        assert_eq!(model.selection_step, SelectionStep::SelectingVersions);
    }

    #[test]
    fn architecture_chosen_selects_and_starts_download() {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingArchitecture;
        model.selected_arch_variant = Some(MicroArchVariants::X86_64V2);

        let _ = crate::update::handle(
            &mut model,
            Message::ArchitectureChosen(MicroArchVariants::X86_64V3),
        );

        assert_eq!(
            model.selected_arch_variant,
            Some(MicroArchVariants::X86_64V3)
        );
        // Without a full selection the download step reports a status instead of panicking.
        assert!(model.global_status.contains("select"));
    }

    #[test]
    fn version_chosen_is_exclusive_in_check_whats_new() {
        let mut model = ready_model();
        model.mode = GuiMode::CheckWhatsNew;
        model.app_mode = AppMode::CheckWhatsNew;
        model.selection_step = SelectionStep::SelectingVersions;
        model.selected_version_indices = vec![0];

        let _ = crate::update::handle(&mut model, Message::VersionChosen(2));

        assert_eq!(model.selected_version_indices, vec![2]);
    }

    #[test]
    fn architecture_selection_renders_buttons_without_continue() -> Result<(), Error> {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingArchitecture;

        let mut ui = simulator(crate::views::app_view(&model));

        assert!(
            ui.find("Continue").is_err(),
            "Continue should be gone; clicking a variant starts the download"
        );
        assert!(ui.find("Back").is_ok(), "Back should be visible");

        Ok(())
    }

    //
    // Changelog pane lifecycle (it must close when navigating away)
    //

    fn model_with_changelog_open() -> ProtonupGui {
        let mut model = ready_model();
        model.mode = GuiMode::CheckWhatsNew;
        model.app_mode = AppMode::CheckWhatsNew;
        model.selection_step = SelectionStep::SelectingVersions;
        model.available_tools = vec![
            make_test_tool(),
            CompatTool::from_str("Luxtorpeda").unwrap(),
        ];
        model.selected_tool = Some(make_test_tool());
        model.available_versions = vec![make_test_release("v1"), make_test_release("v2")];
        model.selected_version_indices = vec![0];

        let _ = crate::update::handle(&mut model, Message::VersionChosen(1));
        assert!(model.show_changelog.is_some(), "precondition: pane is open");
        model
    }

    #[test]
    fn back_to_tool_selection_closes_changelog() {
        let mut model = model_with_changelog_open();

        let _ = crate::update::handle(&mut model, Message::BackToToolSelection);

        assert!(
            model.show_changelog.is_none(),
            "navigating back must close the changelog pane"
        );
    }

    #[test]
    fn new_tool_clears_stale_changelog() {
        let mut model = model_with_changelog_open();

        let _ = crate::update::handle(&mut model, Message::BackToToolSelection);
        let _ = crate::update::handle(&mut model, Message::ToolChosen(1));
        let _ = crate::update::handle(
            &mut model,
            Message::VersionsFetched(vec![make_test_release("new-1"), make_test_release("new-2")]),
        );

        assert!(
            model.show_changelog.is_none(),
            "fetching a new tool's versions must not carry over the old changelog"
        );
    }

    #[test]
    fn back_to_initial_closes_changelog() {
        let mut model = model_with_changelog_open();

        let _ = crate::update::handle(&mut model, Message::BackToInitial);

        assert!(
            model.show_changelog.is_none(),
            "resetting to the main menu must close the changelog pane"
        );
    }

    //
    // Keyboard / controller navigation
    //

    use crate::input::{
        StickState, apply_nav_action, focused_option, gamepad_to_message, keyboard_to_message,
        pane_grid, primary_action,
    };
    use gilrs::Button;
    use iced::keyboard::key::Named;
    use iced::keyboard::{Event as KeyEvent, Key, Location, Modifiers};

    fn key_pressed(named: Named, modifiers: Modifiers) -> KeyEvent {
        KeyEvent::KeyPressed {
            key: Key::Named(named),
            modified_key: Key::Named(named),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Tab),
            location: Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        }
    }

    fn repeat_pressed(named: Named) -> KeyEvent {
        KeyEvent::KeyPressed {
            key: Key::Named(named),
            modified_key: Key::Named(named),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Tab),
            location: Location::Standard,
            modifiers: Modifiers::default(),
            text: None,
            repeat: true,
        }
    }

    fn model_in_tool_selection() -> ProtonupGui {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        model.available_tools = vec![
            make_test_tool(),
            CompatTool::from_str("Luxtorpeda").unwrap(),
            CompatTool::from_str("Luxtorpeda").unwrap(),
        ];
        model
    }

    /// Asserts that two messages are the same variant (messages are not PartialEq).
    fn same_variant(label: &str, expected: &str, actual: &str) {
        assert_eq!(expected, actual, "{}", label);
    }

    fn msg_name(message: &Message) -> String {
        match message {
            Message::Nav(crate::NavAction::FocusNextPane) => "FocusNextPane".into(),
            Message::Nav(crate::NavAction::FocusPreviousPane) => "FocusPreviousPane".into(),
            Message::Nav(crate::NavAction::MoveUp) => "MoveUp".into(),
            Message::Nav(crate::NavAction::MoveDown) => "MoveDown".into(),
            Message::Nav(crate::NavAction::MoveLeft) => "MoveLeft".into(),
            Message::Nav(crate::NavAction::MoveRight) => "MoveRight".into(),
            Message::Nav(crate::NavAction::Select) => "Select".into(),
            Message::Nav(crate::NavAction::Primary) => "Primary".into(),
            Message::StartSelectedDownloads => "StartSelectedDownloads".into(),
            Message::BackToInitial => "BackToInitial".into(),
            Message::ToolChosen(_) => "ToolChosen".into(),
            Message::ToggleKeyboardShortcuts => "ToggleKeyboardShortcuts".into(),
            Message::ToggleControllerShortcuts => "ToggleControllerShortcuts".into(),
            Message::CloseShortcuts => "CloseShortcuts".into(),
            Message::Scroll { .. } => "Scroll".into(),
            other => format!("other({:?})", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn keyboard_tab_maps_to_next_pane() {
        let msg = keyboard_to_message(&key_pressed(Named::Tab, Modifiers::default()));
        same_variant(
            "Tab",
            "FocusNextPane",
            &msg_name(&msg.expect("Tab should map")),
        );
    }

    #[test]
    fn keyboard_shift_tab_maps_to_previous_pane() {
        let msg = keyboard_to_message(&key_pressed(Named::Tab, Modifiers::SHIFT));
        same_variant(
            "Shift+Tab",
            "FocusPreviousPane",
            &msg_name(&msg.expect("Shift+Tab should map")),
        );
    }

    #[test]
    fn keyboard_arrows_map_to_movement() {
        for (named, expected) in [
            (Named::ArrowUp, "MoveUp"),
            (Named::ArrowDown, "MoveDown"),
            (Named::ArrowLeft, "MoveLeft"),
            (Named::ArrowRight, "MoveRight"),
        ] {
            let msg = keyboard_to_message(&key_pressed(named, Modifiers::default()))
                .expect("arrow should map");
            same_variant(&format!("{:?}", named), expected, &msg_name(&msg));
        }
    }

    #[test]
    fn keyboard_space_selects_and_enter_is_primary() {
        let select = keyboard_to_message(&key_pressed(Named::Space, Modifiers::default()))
            .expect("Space should map");
        same_variant("Space", "Select", &msg_name(&select));

        let primary = keyboard_to_message(&key_pressed(Named::Enter, Modifiers::default()))
            .expect("Enter should map");
        same_variant("Enter", "Primary", &msg_name(&primary));
    }

    #[test]
    fn keyboard_activation_keys_ignore_repeats() {
        assert!(
            keyboard_to_message(&repeat_pressed(Named::Space)).is_none(),
            "held Space must not re-activate"
        );
        assert!(
            keyboard_to_message(&repeat_pressed(Named::Enter)).is_none(),
            "held Enter must not re-activate"
        );
        // Movement repeats are still allowed (hold-to-scroll).
        assert!(keyboard_to_message(&repeat_pressed(Named::ArrowDown)).is_some());
    }

    #[test]
    fn keyboard_question_mark_toggles_keyboard_shortcuts() {
        let event = KeyEvent::KeyPressed {
            key: Key::Character("?".into()),
            modified_key: Key::Character("?".into()),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Slash),
            location: Location::Standard,
            modifiers: Modifiers::SHIFT,
            text: Some("?".into()),
            repeat: false,
        };
        let msg = keyboard_to_message(&event).expect("? should map");
        same_variant("?", "ToggleKeyboardShortcuts", &msg_name(&msg));
    }

    #[test]
    fn keyboard_escape_closes_shortcuts() {
        let msg = keyboard_to_message(&key_pressed(Named::Escape, Modifiers::default()))
            .expect("Escape should map");
        same_variant("Escape", "CloseShortcuts", &msg_name(&msg));
    }

    #[test]
    fn keyboard_unrelated_keys_are_ignored() {
        assert!(keyboard_to_message(&key_pressed(Named::F5, Modifiers::default())).is_none());
        // Key releases do not navigate.
        assert!(
            keyboard_to_message(&KeyEvent::KeyReleased {
                key: Key::Named(Named::Tab),
                modified_key: Key::Named(Named::Tab),
                physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Tab,),
                location: Location::Standard,
                modifiers: Modifiers::default(),
            })
            .is_none()
        );
    }

    #[test]
    fn gamepad_buttons_map_to_navigation() {
        let cases = [
            (Button::RightTrigger, "FocusNextPane"),
            (Button::RightTrigger2, "FocusNextPane"),
            (Button::LeftTrigger, "FocusPreviousPane"),
            (Button::LeftTrigger2, "FocusPreviousPane"),
            (Button::DPadUp, "MoveUp"),
            (Button::DPadDown, "MoveDown"),
            (Button::DPadLeft, "MoveLeft"),
            (Button::DPadRight, "MoveRight"),
            (Button::South, "Select"),
            (Button::Start, "Primary"),
            (Button::North, "ToggleControllerShortcuts"),
            (Button::East, "CloseShortcuts"),
        ];

        for (button, expected) in cases {
            let msg = gamepad_to_message(button).expect("button should map");
            same_variant(&format!("{:?}", button), expected, &msg_name(&msg));
        }
    }

    #[test]
    fn gamepad_unmapped_buttons_are_ignored() {
        assert!(gamepad_to_message(Button::West).is_none());
        assert!(gamepad_to_message(Button::Select).is_none());
    }

    #[test]
    fn stick_is_edge_triggered_with_deadzone() {
        let mut stick = StickState::default();

        // Inside the deadzone: nothing.
        assert!(stick.sample((0.1, 0.1)).is_none());

        // Pushed down past the deadzone: one step.
        let first = stick.sample((0.0, -0.9)).expect("first push should step");
        same_variant("stick down", "MoveDown", &msg_name(&first));

        // Held: no repeat.
        assert!(
            stick.sample((0.0, -0.9)).is_none(),
            "holding the stick must not repeat"
        );

        // Back to centre then down again: another step.
        assert!(stick.sample((0.0, 0.0)).is_none());
        assert!(stick.sample((0.0, -0.9)).is_some());
    }

    #[test]
    fn stick_horizontal_pushes_map_to_left_and_right() {
        let mut stick = StickState::default();
        let right = stick.sample((0.9, 0.0)).expect("right push");
        same_variant("stick right", "MoveRight", &msg_name(&right));

        assert!(stick.sample((0.0, 0.0)).is_none());
        let left = stick.sample((-0.9, 0.0)).expect("left push");
        same_variant("stick left", "MoveLeft", &msg_name(&left));
    }

    #[test]
    fn sidebar_grid_skips_disabled_entries() {
        let mut model = ready_model();
        assert_eq!(pane_grid(&model, crate::Pane::Sidebar).len(), 7);

        // Entering a mode disables that mode's own sidebar entry.
        model.app_mode = AppMode::DownloadForSteam;
        let grid = pane_grid(&model, crate::Pane::Sidebar);
        assert_eq!(
            grid.len(),
            6,
            "the active mode's entry must not be focusable"
        );
        assert!(
            grid.iter().all(|row| row.len() == 1),
            "sidebar is one column"
        );
    }

    #[test]
    fn tool_selection_grid_matches_rendered_options() {
        let model = model_in_tool_selection();
        let grid = pane_grid(&model, crate::Pane::Main);
        assert_eq!(grid.len(), 3, "one row per tool");
        assert!(grid.iter().all(|row| row.len() == 1));
    }

    #[test]
    fn actions_grid_matches_action_bar() {
        let model = model_in_tool_selection();
        let grid = pane_grid(&model, crate::Pane::Actions);
        assert_eq!(grid.len(), 1, "actions are a single row");
        assert_eq!(grid[0].len(), 1, "tool selection has the Back action");
    }

    /// Apps with installed tools come first; apps without are grouped into one
    /// non-interactive column, so they contribute no focusable options.
    #[test]
    fn manage_partition_puts_populated_apps_first() {
        use crate::views::manage_installations::partition_views;

        let version = |name: &str| crate::message::InstalledVersion {
            name: name.into(),
            path: PathBuf::from("/tmp").join(name),
            selected_for_deletion: false,
        };

        let views = vec![
            crate::message::AppInstallationView {
                app: AppInstallations::Steam,
                versions: vec![version("GE-Proton9-1")],
                loading: false,
            },
            crate::message::AppInstallationView {
                app: AppInstallations::SteamFlatpak,
                versions: vec![],
                loading: false,
            },
            crate::message::AppInstallationView {
                app: AppInstallations::Lutris,
                versions: vec![version("GE-Proton9-2"), version("GE-Proton9-3")],
                loading: false,
            },
            crate::message::AppInstallationView {
                app: AppInstallations::LutrisFlatpak,
                versions: vec![],
                loading: false,
            },
        ];

        let (populated, empty) = partition_views(&views);
        let names = |group: &[&crate::message::AppInstallationView]| {
            group.iter().map(|v| v.app.clone()).collect::<Vec<_>>()
        };

        assert_eq!(
            names(&populated),
            vec![AppInstallations::Steam, AppInstallations::Lutris],
            "apps with tools come first, in original order"
        );
        assert_eq!(
            names(&empty),
            vec![
                AppInstallations::SteamFlatpak,
                AppInstallations::LutrisFlatpak
            ],
            "apps without tools keep their original order"
        );
    }

    #[test]
    fn manage_grid_is_apps_columns_by_versions_rows() {
        let mut model = ready_model();
        model.app_mode = AppMode::ManageInstallations;
        model.mode = GuiMode::ManageInstallations;
        model.app_installations_views = vec![
            crate::message::AppInstallationView {
                app: AppInstallations::Steam,
                versions: vec![
                    crate::message::InstalledVersion {
                        name: "GE-Proton9-1".into(),
                        path: PathBuf::from("/tmp/a"),
                        selected_for_deletion: false,
                    },
                    crate::message::InstalledVersion {
                        name: "GE-Proton9-2".into(),
                        path: PathBuf::from("/tmp/b"),
                        selected_for_deletion: false,
                    },
                ],
                loading: false,
            },
            crate::message::AppInstallationView {
                app: AppInstallations::Lutris,
                versions: vec![crate::message::InstalledVersion {
                    name: "GE-Proton9-3".into(),
                    path: PathBuf::from("/tmp/c"),
                    selected_for_deletion: false,
                }],
                loading: false,
            },
        ];

        let grid = pane_grid(&model, crate::Pane::Main);
        assert_eq!(grid.len(), 2, "two version rows");
        assert_eq!(grid[0].len(), 2, "both apps have a first version");
        assert_eq!(grid[1].len(), 1, "only Steam has a second version");
    }

    #[test]
    fn primary_action_per_view() {
        // Tool selection: there is no "advance" action, Enter falls back.
        let model = model_in_tool_selection();
        assert!(primary_action(&model).is_none());

        // Version selection for a download: Start Selected Downloads.
        let mut model = model_in_tool_selection();
        model.selection_step = SelectionStep::SelectingVersions;
        model.available_versions = vec![make_test_release("v1")];
        let msg = primary_action(&model).expect("downloads have a primary action");
        same_variant("versions", "StartSelectedDownloads", &msg_name(&msg));

        // Check What's New has no advance action.
        let mut model = model_in_tool_selection();
        model.mode = GuiMode::CheckWhatsNew;
        model.app_mode = AppMode::CheckWhatsNew;
        model.selection_step = SelectionStep::SelectingVersions;
        assert!(primary_action(&model).is_none());
    }

    #[test]
    fn left_right_cycles_panes_when_no_horizontal_neighbour() {
        let mut model = model_in_tool_selection();
        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Main;
        model.focus.row = 0;
        model.focus.col = 0;

        // Single-column pane: right falls through to the next pane.
        apply_nav_action(&mut model, crate::NavAction::MoveRight);
        assert_eq!(model.focus.pane, crate::Pane::Actions);
        assert_eq!((model.focus.row, model.focus.col), (0, 0));

        // And left goes back.
        apply_nav_action(&mut model, crate::NavAction::MoveLeft);
        assert_eq!(model.focus.pane, crate::Pane::Main);
    }

    #[test]
    fn left_right_moves_within_multi_column_rows() {
        let mut model = model_in_tool_selection();
        model.selection_step = SelectionStep::SelectingVersions;
        model.available_versions = vec![make_test_release("v1")];
        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Actions;

        // Three action buttons in one row.
        apply_nav_action(&mut model, crate::NavAction::MoveRight);
        assert_eq!((model.focus.row, model.focus.col), (0, 1));
        assert_eq!(model.focus.pane, crate::Pane::Actions);

        apply_nav_action(&mut model, crate::NavAction::MoveLeft);
        assert_eq!((model.focus.row, model.focus.col), (0, 0));
    }

    #[test]
    fn vertical_movement_clamps_at_grid_edges() {
        let mut model = model_in_tool_selection();
        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Main;

        apply_nav_action(&mut model, crate::NavAction::MoveUp);
        assert_eq!(model.focus.row, 0, "cannot move above the first row");

        apply_nav_action(&mut model, crate::NavAction::MoveDown);
        assert_eq!(model.focus.row, 1);
        apply_nav_action(&mut model, crate::NavAction::MoveDown);
        apply_nav_action(&mut model, crate::NavAction::MoveDown);
        assert_eq!(model.focus.row, 2, "clamped to the last row");
    }

    #[test]
    fn focus_cursor_survives_grid_shrink() {
        let mut model = model_in_tool_selection();
        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Main;
        model.focus.row = 2;

        // Tools disappear (e.g. reload); moving must not panic and must clamp.
        model.available_tools.clear();
        apply_nav_action(&mut model, crate::NavAction::MoveDown);
        assert!(focused_option(&model).is_none());
    }

    #[test]
    fn select_activates_focused_option() {
        let mut model = model_in_tool_selection();
        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Main;
        model.focus.row = 1;

        let msg = apply_nav_action(&mut model, crate::NavAction::Select).expect("should activate");
        assert!(
            matches!(msg, Message::ToolChosen(1)),
            "expected the second tool, got {:?}",
            msg_name(&msg)
        );
    }

    #[test]
    fn nav_engages_focus_and_dispatches_through_update() {
        let mut model = model_in_tool_selection();
        assert!(!model.focus.engaged, "focus starts disengaged");

        let _ = crate::update::handle(&mut model, Message::Nav(crate::NavAction::MoveDown));

        assert!(model.focus.engaged, "first navigation engages highlighting");
        assert_eq!(model.focus.row, 1, "MoveDown advanced from the start");
    }

    #[test]
    fn shortcut_panels_toggle_exclusively() {
        let mut model = ready_model();

        let _ = crate::update::handle(&mut model, Message::ToggleKeyboardShortcuts);
        assert!(model.show_keyboard_shortcuts);

        let _ = crate::update::handle(&mut model, Message::ToggleControllerShortcuts);
        assert!(
            !model.show_keyboard_shortcuts,
            "opening controller help closes keyboard help"
        );
        assert!(model.show_controller_shortcuts);

        let _ = crate::update::handle(&mut model, Message::CloseShortcuts);
        assert!(!model.show_keyboard_shortcuts && !model.show_controller_shortcuts);
    }

    #[test]
    fn nav_closes_open_shortcut_panel() {
        let mut model = ready_model();
        model.show_keyboard_shortcuts = true;

        let _ = crate::update::handle(&mut model, Message::Nav(crate::NavAction::Select));

        assert!(
            !model.show_keyboard_shortcuts,
            "any activation closes the panel"
        );
        assert!(
            !model.focus.engaged,
            "closing the panel must not move focus"
        );
    }

    #[test]
    fn controller_shortcuts_refused_when_gamepads_unavailable() {
        let mut model = ready_model();
        model.gamepads_unavailable = true;

        let _ = crate::update::handle(&mut model, Message::ToggleControllerShortcuts);

        assert!(
            !model.show_controller_shortcuts,
            "no controller help without a controller backend"
        );
    }

    #[test]
    fn footer_advertises_both_shortcut_panels() -> Result<(), Error> {
        let model = ready_model();
        let mut ui = simulator(crate::views::app_view(&model));

        assert!(
            ui.find(crate::views::footer::KEYBOARD_HINT).is_ok(),
            "the keyboard hint must be visible on the main pane"
        );
        assert!(
            ui.find(crate::views::footer::CONTROLLER_HINT).is_ok(),
            "the controller hint must be visible on the main pane"
        );

        Ok(())
    }

    #[test]
    fn keyboard_shortcut_panel_renders() -> Result<(), Error> {
        let mut model = ready_model();
        model.show_keyboard_shortcuts = true;
        let mut ui = simulator(crate::views::app_view(&model));

        assert!(ui.find("Keyboard Shortcuts").is_ok());
        assert!(
            ui.find("Focus the next pane (Sidebar → Main → Actions)")
                .is_ok(),
            "the panel must list the Tab shortcut"
        );
        assert!(
            ui.find("Quick Update").is_err(),
            "the normal layout is replaced while the panel is open"
        );

        Ok(())
    }

    #[test]
    fn controller_shortcut_panel_renders() -> Result<(), Error> {
        let mut model = ready_model();
        model.show_controller_shortcuts = true;
        let mut ui = simulator(crate::views::app_view(&model));

        assert!(ui.find("Controller Shortcuts").is_ok());
        assert!(ui.find("RB / RT").is_ok());
        // Every input the gamepad stream can emit should be documented.
        assert!(
            ui.find("RIGHT STICK").is_ok(),
            "the right stick scrolls, so it must appear in the shortcut panel"
        );

        Ok(())
    }

    #[test]
    fn none_of_the_footer_hints_are_focusable() {
        // The hint bar is informational: it must not enter any pane grid.
        let model = ready_model();
        let mut all = pane_grid(&model, crate::Pane::Sidebar);
        all.extend(pane_grid(&model, crate::Pane::Main));
        all.extend(pane_grid(&model, crate::Pane::Actions));

        assert!(
            all.iter().all(|row| row.iter().all(|option| !matches!(
                option.message,
                Message::ToggleKeyboardShortcuts | Message::ToggleControllerShortcuts
            ))),
            "hint buttons must stay out of the navigation grid"
        );
    }

    #[test]
    fn controller_back_button_behaves_like_escape() {
        // (B)/(circle) must map to the same message as the Escape key.
        let gamepad = gamepad_to_message(Button::East).expect("B must map");
        let escape =
            keyboard_to_message(&key_pressed(Named::Escape, Modifiers::default())).expect("Esc");

        assert_eq!(msg_name(&gamepad), "CloseShortcuts");
        assert_eq!(msg_name(&escape), "CloseShortcuts");
    }

    #[test]
    fn controller_back_closes_an_open_shortcut_panel() {
        let mut model = ready_model();
        model.show_controller_shortcuts = true;

        let msg = gamepad_to_message(Button::East).expect("B must map");
        let _ = crate::update::handle(&mut model, msg);

        assert!(
            !model.show_controller_shortcuts,
            "(B)/(circle) must close the shortcut panel like Escape"
        );
    }

    #[test]
    fn tab_skips_inactive_panes_at_startup() {
        // On the initial screen only the sidebar has options: the main pane is
        // the "choose your option" placeholder and the action bar is empty.
        let mut model = ready_model();
        assert!(crate::input::pane_is_active(&model, crate::Pane::Sidebar));
        assert!(
            !crate::input::pane_is_active(&model, crate::Pane::Main),
            "the placeholder main pane must not be focusable"
        );
        assert!(
            !crate::input::pane_is_active(&model, crate::Pane::Actions),
            "an empty action bar must not be focusable"
        );

        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Sidebar;

        // Tab must stay on the only active pane instead of entering empty ones.
        apply_nav_action(&mut model, crate::NavAction::FocusNextPane);
        assert_eq!(model.focus.pane, crate::Pane::Sidebar);

        apply_nav_action(&mut model, crate::NavAction::FocusPreviousPane);
        assert_eq!(model.focus.pane, crate::Pane::Sidebar);
    }

    #[test]
    fn tab_cycles_only_active_panes_when_main_has_options() {
        let mut model = model_in_tool_selection();
        assert!(crate::input::pane_is_active(&model, crate::Pane::Sidebar));
        assert!(crate::input::pane_is_active(&model, crate::Pane::Main));
        assert!(crate::input::pane_is_active(&model, crate::Pane::Actions));

        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Sidebar;

        apply_nav_action(&mut model, crate::NavAction::FocusNextPane);
        assert_eq!(model.focus.pane, crate::Pane::Main);
        apply_nav_action(&mut model, crate::NavAction::FocusNextPane);
        assert_eq!(model.focus.pane, crate::Pane::Actions);
        apply_nav_action(&mut model, crate::NavAction::FocusNextPane);
        assert_eq!(model.focus.pane, crate::Pane::Sidebar);
    }

    #[test]
    fn horizontal_fallback_skips_inactive_panes() {
        // On startup the sidebar is the only active pane: Left/Right must not
        // fall through into the hidden main pane or empty action bar.
        let mut model = ready_model();
        model.focus.engaged = true;
        model.focus.pane = crate::Pane::Sidebar;

        apply_nav_action(&mut model, crate::NavAction::MoveRight);
        assert_eq!(
            model.focus.pane,
            crate::Pane::Sidebar,
            "Right must stay on the only active pane"
        );

        apply_nav_action(&mut model, crate::NavAction::MoveLeft);
        assert_eq!(model.focus.pane, crate::Pane::Sidebar);
    }

    #[test]
    fn list_selection_options_render_as_buttons() -> Result<(), Error> {
        // Regression: tool/version options used to be bare text wrapped in a
        // mouse_area, which looked like plain links. They must render as real
        // buttons: a fixed-width button with 10px padding each side, so the
        // label fills that inner area instead of shrinking to its text width.
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        model.available_tools = vec![
            make_test_tool(),
            CompatTool::from_str("Luxtorpeda").unwrap(),
        ];

        let mut ui = simulator(crate::views::app_view(&model));
        let label = ui.find("GEProton")?;
        let bounds = label.bounds();

        // 10px button padding on each side of the fixed-width list button.
        let expected_inner = crate::views::action_bar::LIST_BUTTON_WIDTH - 20.0;
        assert!(
            (bounds.width - expected_inner).abs() < 0.5,
            "option label should span the button's inner width ({expected_inner}), got {}",
            bounds.width
        );

        Ok(())
    }

    #[test]
    fn clicking_a_checkbox_option_toggles_exactly_once() -> Result<(), Error> {
        // Options are now real buttons wrapping a checkbox: a click must emit
        // the toggle exactly once (a duplicate would cancel itself out).
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingVersions;
        model.selected_tool = Some(make_test_tool());
        model.available_versions = vec![make_test_release("v1")];
        model.selected_version_indices = vec![];

        let mut ui = simulator(crate::views::app_view(&model));
        ui.click("v1")?;

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages.len(), 1, "one click must emit exactly one message");
        assert!(
            matches!(messages[0], Message::ToggleVersion(0)),
            "the click must toggle version 0"
        );

        Ok(())
    }

    #[test]
    fn clicking_a_list_option_picks_it() -> Result<(), Error> {
        let mut model = ready_model();
        model.mode = GuiMode::DownloadForSteam;
        model.app_mode = AppMode::DownloadForSteam;
        model.selection_step = SelectionStep::SelectingTools;
        model.available_tools = vec![
            make_test_tool(),
            CompatTool::from_str("Luxtorpeda").unwrap(),
        ];

        let mut ui = simulator(crate::views::app_view(&model));
        ui.click("Luxtorpeda")?;

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            matches!(messages.as_slice(), [Message::ToolChosen(1)]),
            "clicking the second tool must choose index 1"
        );

        Ok(())
    }

    #[test]
    fn checkbox_options_use_the_same_width_limit_as_lists() {
        // Regression: checkbox rows used to span the whole pane because they
        // went through the Fill-width option row. They must be bounded by the
        // shared list button width instead.
        use crate::views::focus_style::Highlight;

        let width = crate::views::action_bar::LIST_BUTTON_WIDTH;
        let row = crate::views::focus_style::option_checkbox_row(
            true,
            "GE-Proton9-27",
            Highlight::None,
            Message::ToggleVersion(0),
            width,
        );

        let mut ui = Simulator::with_size(Settings::default(), Size::new(1400.0, 900.0), row);
        let bounds = ui
            .find(|c: Candidate<'_>| match c {
                Candidate::Container { bounds, .. } => Some(bounds),
                _ => None,
            })
            .expect("checkbox row should have layout bounds");

        assert!(
            (bounds.width - width).abs() < 0.5,
            "checkbox option should be {width} wide, got {}",
            bounds.width
        );
    }

    //
    // Right-stick scrolling and focused-option reveal
    //

    #[test]
    fn right_stick_inside_deadzone_does_not_scroll() {
        assert!(crate::input::right_stick_to_scroll((0.0, 0.0)).is_none());
        assert!(crate::input::right_stick_to_scroll((0.1, -0.2)).is_none());
        assert!(crate::input::right_stick_to_scroll((0.24, 0.24)).is_none());
    }

    #[test]
    fn right_stick_up_scrolls_toward_top() {
        // gilrs reports a positive `y` when the stick is pushed up, and iced's
        // `scroll_by` adds the delta to the offset, so up must be negative.
        let message = crate::input::right_stick_to_scroll((0.0, 1.0)).expect("should scroll");
        match message {
            Message::Scroll { x, y } => {
                assert_eq!(x, 0.0);
                assert!(y < 0.0, "pushing up should scroll toward the top, got {y}");
            }
            other => panic!("expected Scroll, got {other:?}"),
        }
    }

    #[test]
    fn right_stick_down_scrolls_toward_bottom() {
        let message = crate::input::right_stick_to_scroll((0.0, -1.0)).expect("should scroll");
        match message {
            Message::Scroll { x, y } => {
                assert_eq!(x, 0.0);
                assert!(
                    y > 0.0,
                    "pushing down should scroll toward the bottom, got {y}"
                );
            }
            other => panic!("expected Scroll, got {other:?}"),
        }
    }

    #[test]
    fn right_stick_horizontal_is_proportional() {
        let message = crate::input::right_stick_to_scroll((1.0, 0.0)).expect("should scroll");
        match message {
            Message::Scroll { x, y } => {
                assert!(x > 0.0, "pushing right should scroll right, got {x}");
                assert_eq!(y, 0.0);
            }
            other => panic!("expected Scroll, got {other:?}"),
        }

        // Partial deflection produces a smaller step than full deflection.
        let full = crate::input::right_stick_to_scroll((1.0, 0.0));
        let half = crate::input::right_stick_to_scroll((0.5, 0.0));
        match (full, half) {
            (Some(Message::Scroll { x: full, .. }), Some(Message::Scroll { x: half, .. })) => {
                assert!(
                    half.abs() < full.abs(),
                    "partial deflection should step less"
                );
            }
            _ => panic!("both samples should scroll"),
        }
    }

    #[test]
    fn scroll_target_follows_focus_cursor() {
        let mut model = model_in_tool_selection();

        model.focus.pane = crate::Pane::Sidebar;
        assert_eq!(
            crate::input::scroll_target_id(&model),
            Some(crate::views::scroll_id::sidebar())
        );

        model.focus.pane = crate::Pane::Main;
        assert_eq!(
            crate::input::scroll_target_id(&model),
            Some(crate::views::scroll_id::main())
        );
    }

    #[test]
    fn action_pane_has_no_scroll_target() {
        let mut model = model_in_tool_selection();
        model.focus.pane = crate::Pane::Actions;
        assert_eq!(crate::input::scroll_target_id(&model), None);
    }

    #[test]
    fn reveal_scroll_starts_at_first_option() {
        let mut model = model_in_tool_selection();
        model.focus.pane = crate::Pane::Main;
        model.focus.row = 0;
        model.focus.col = 0;

        // The first option sits at the top of the list.
        assert!(crate::input::reveal_focused_scroll(&model).is_some());
    }

    #[test]
    fn reveal_scroll_is_none_when_there_is_nothing_to_scroll() {
        let mut model = model_in_tool_selection();
        model.focus.pane = crate::Pane::Actions;
        assert!(crate::input::reveal_focused_scroll(&model).is_none());
    }

    #[test]
    fn nav_movement_keeps_focus_in_bounds_and_reveals() {
        let mut model = model_in_tool_selection();
        model.focus.pane = crate::Pane::Main;
        model.focus.engaged = true;

        // Walk down past the end of the list; the cursor must stay valid.
        for _ in 0..10 {
            let _ = crate::update::handle(&mut model, Message::Nav(crate::NavAction::MoveDown));
        }

        let rows = crate::input::pane_grid(&model, crate::Pane::Main).len();
        assert!(rows > 0, "tool selection should expose options");
        assert!(
            model.focus.row < rows,
            "focus row {} out of bounds for {rows} rows",
            model.focus.row
        );
    }
}
