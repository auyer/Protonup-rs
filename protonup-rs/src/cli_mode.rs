use anyhow::Error;
use libprotonup::apps::{App, AppInstallations};
use libprotonup::downloads::{self, Release};
use libprotonup::sources::CompatTool;
use libprotonup::utils::match_version;

use crate::architecture_variants;
use crate::download;

/// Determines the target application based on the provided `--for` argument and the selected tool.
///
/// - If the value is "steam" (case-insensitive), uses `App::Steam`
/// - If the value is "lutris" (case-insensitive), uses `App::Lutris`
/// - If the value is a file system path (relative or absolute), uses `App::Custom`
/// - If the value is None, auto-detects based on:
///   1. Compatible applications for the selected tool
///   2. Installed apps (Steam/Lutris)
async fn determine_app_installation(
    for_target: Option<&str>,
    compat_tool: &CompatTool,
) -> Result<AppInstallations, Error> {
    let app = match for_target {
        Some(target) => App::from_str_or_path(target),
        None => {
            // Auto-detect based on tool's compatible applications
            return auto_detect_app(&compat_tool.compatible_applications).await;
        }
    };

    // For Steam and Lutris, detect installation method; for Custom, use directly
    match app {
        App::Steam => {
            let apps = App::Steam.detect_installation_method().await;
            if apps.is_empty() {
                return Err(anyhow::anyhow!(
                    "Steam installation not found. Install location for Steam not found."
                ));
            }
            Ok(apps[0].clone())
        }
        App::Lutris => {
            let apps = App::Lutris.detect_installation_method().await;
            if apps.is_empty() {
                return Err(anyhow::anyhow!(
                    "Lutris installation not found. Install location for Lutris not found."
                ));
            }
            Ok(apps[0].clone())
        }
        App::Custom(path) => Ok(AppInstallations::new_custom_app_install(path)),
    }
}

/// Auto-detects the best app installation based on compatible applications and what's installed.
async fn auto_detect_app(compatible_apps: &[App]) -> Result<AppInstallations, Error> {
    // Check compatible apps in order of preference
    for compat_app in compatible_apps {
        match compat_app {
            App::Steam => {
                let apps = App::Steam.detect_installation_method().await;
                if !apps.is_empty() {
                    return Ok(apps[0].clone());
                }
            }
            App::Lutris => {
                let apps = App::Lutris.detect_installation_method().await;
                if !apps.is_empty() {
                    return Ok(apps[0].clone());
                }
            }
            App::Custom(path) => {
                return Ok(AppInstallations::new_custom_app_install(path.clone()));
            }
        }
    }

    // If no compatible apps are installed, provide a helpful error
    let compatible_names: Vec<String> = compatible_apps
        .iter()
        .filter(|app| !matches!(app, App::Custom(_)))
        .map(|app| app.to_string())
        .collect();

    Err(anyhow::anyhow!(
        "{} installation(s) not found. Use --for to specify 'steam', 'lutris', or a custom installation path.",
        compatible_names.join(" and ")
    ))
}

/// Finds a release matching the user-provided version string.
fn find_release_by_version(
    release_list: Vec<Release>,
    version_str: &str,
    tool_name: &str,
) -> Result<Release, Error> {
    let available_versions: Vec<String> = release_list.iter().map(|r| r.tag_name.clone()).collect();

    // Try to find a matching release
    let matching_release = release_list
        .into_iter()
        .find(|r| match_version(version_str, &r.tag_name));

    match matching_release {
        Some(release) => Ok(release),
        None => Err(anyhow::anyhow!(
            "Version '{}' not found for {}. Available versions: {}",
            version_str,
            tool_name,
            available_versions.join(", ")
        )),
    }
}

/// Runs the program in CLI mode with provided arguments.
/// This is a non-interactive mode that downloads the specified tool/version.
///
/// # Arguments
/// * `tool` - Compatibility tool name (e.g., "GEProton", "WineGE")
/// * `version` - Version to install (use "latest" for the latest version)
/// * `for_target` - Target for installation: "steam", "lutris", or a custom path. None for auto-detect.
/// * `force` - Force overwrite existing installations
pub async fn run_cli_mode(
    tool: Option<String>,
    version: Option<String>,
    for_target: Option<String>,
    force: bool,
) -> Result<Vec<Release>, Error> {
    // Determine the compatibility tool first (needed for auto-detection)
    let compat_tool = match tool.as_deref() {
        Some(tool_name) => tool_name.parse::<CompatTool>().map_err(|_| {
            anyhow::anyhow!(
                "Unknown compatibility tool: '{}'. Available tools: {}",
                tool_name,
                libprotonup::sources::CompatTools
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?,
        None => {
            // If no tool specified, determine app first, then use its default tool
            let temp_app = match for_target.as_deref() {
                Some(target) => App::from_str_or_path(target),
                None => {
                    // Auto-detect: prefer Steam, fallback to Lutris
                    let apps = App::Steam.detect_installation_method().await;
                    if !apps.is_empty() {
                        App::Steam
                    } else {
                        App::Lutris
                    }
                }
            };
            temp_app.default_compatibility_tool()
        }
    };

    // Determine the target app installation (uses compat_tool's compatible_applications for auto-detect)
    let app_inst = determine_app_installation(for_target.as_deref(), &compat_tool).await?;

    // Get the releases
    let release_list = match downloads::list_releases(&compat_tool).await {
        Ok(list) => list,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to fetch releases for {}: {}",
                compat_tool.name,
                e
            ));
        }
    };

    // Select the version
    let releases =
        match version.as_deref() {
            Some("latest") | None => {
                // Use the latest version (first in the list)
                vec![release_list.into_iter().next().ok_or_else(|| {
                    anyhow::anyhow!("No releases available for {}", compat_tool.name)
                })?]
            }
            Some(version_str) => {
                // Find the matching version using flexible matching
                vec![find_release_by_version(
                    release_list,
                    version_str,
                    &compat_tool.name,
                )?]
            }
        };

    // Handle tools with multiple asset variations
    let downloads_vec: Vec<downloads::Download> = if compat_tool.has_multiple_asset_variations {
        releases
            .iter()
            .map(|release| {
                let variants = release.get_all_download_variants(&app_inst, &compat_tool);
                architecture_variants::select_architecture_variant(
                    &release.tag_name,
                    variants,
                    false,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Error selecting architecture variant: {}", e);
                    std::process::exit(1);
                })
            })
            .collect()
    } else {
        releases
            .iter()
            .map(|release| release.get_download_info(&app_inst, &compat_tool))
            .collect()
    };

    // Download, validate, and unpack
    let multi_progress = indicatif::MultiProgress::with_draw_target(
        indicatif::ProgressDrawTarget::stderr_with_hz(20),
    );

    for download_item in downloads_vec {
        let install_dir = app_inst.installation_dir(&compat_tool).unwrap();
        let file = download::download_file(&download_item, multi_progress.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Error downloading {}: {}", download_item.version, e))?;

        // Validate hash if available
        if let Some(ref git_hash_sum) = download_item.hash_sum {
            let hash_content =
                &downloads::download_file_into_memory(&git_hash_sum.sum_content).await?;
            let hash_sum = libprotonup::hashing::HashSums {
                sum_content: hash_content.to_owned(),
                sum_type: git_hash_sum.sum_type.clone(),
            };
            download::validate_file(
                &download_item.file_name,
                &file,
                hash_sum,
                multi_progress.clone(),
            )
            .await?;
        }

        // Install
        let install_name = compat_tool.installation_name(&download_item.version);
        let install_path = install_dir.join(install_name.clone());
        if libprotonup::files::check_if_exists(&install_path).await && !force {
            return Err(anyhow::anyhow!(
                "Version {} already exists at {}. Use --force to overwrite.",
                install_name,
                install_path.display()
            ));
        }
        if libprotonup::files::check_if_exists(&install_path).await {
            tokio::fs::remove_dir_all(&install_path).await?;
        }

        let unpack_progress_bar =
            download::init_unpack_progress(&install_dir.clone(), &file, multi_progress.clone())
                .await?;

        let decompressor = libprotonup::files::Decompressor::from_path(&file)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Error checking file type of {}: {}", file.display(), e)
            })?;

        libprotonup::files::unpack_file(
            &compat_tool,
            &download_item,
            unpack_progress_bar.wrap_async_read(decompressor),
            &install_dir,
        )
        .await?;

        unpack_progress_bar.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{msg}")
                .unwrap(),
        );
        unpack_progress_bar.finish_with_message(format!(
            "Done! {} installed in {}/{}",
            compat_tool,
            download_item
                .for_app
                .installation_dir(&compat_tool)
                .unwrap()
                .to_str()
                .unwrap(),
            install_name
        ));
    }

    Ok(releases)
}
