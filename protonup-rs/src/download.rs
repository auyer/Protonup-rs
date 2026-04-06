use anyhow::{Context, Result, anyhow, bail};
use futures_util::stream::FuturesUnordered;
use futures_util::{StreamExt, future, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use inquire::{Select, Text};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::BufReader;
use tokio::sync::OnceCell;

use libprotonup::{
    apps,
    downloads::{self, Download, Release},
    files, hashing,
    sources::{CompatTool, CompatTools},
};

use crate::{architecture_variants, file_path, helper_menus};

static PROGRESS_BAR_STYLE: OnceCell<ProgressStyle> = OnceCell::const_new();
static MESSAGE_BAR_STYLE: OnceCell<ProgressStyle> = OnceCell::const_new();

pub(crate) async fn init_download_progress(
    download: &Download,
    tmp_dir: &Path,
    multi_progress: MultiProgress,
) -> ProgressBar {
    let progress_bar = multi_progress.add(ProgressBar::new(download.size));
    progress_bar.set_style(get_progress_style().await);
    progress_bar.set_message(format!(
        "Downloading {} to {}",
        download.download_url.split('/').next_back().unwrap(),
        tmp_dir.display()
    ));

    progress_bar
}

pub(crate) async fn init_hash_progress(
    path: &Path,
    multi_progress: MultiProgress,
) -> Result<ProgressBar> {
    let progress_bar = multi_progress.add(ProgressBar::new(fs::metadata(path).await?.len()));
    progress_bar.set_style(get_progress_style().await);
    progress_bar.set_message(format!("Validating {}", path.display()));
    Ok(progress_bar)
}

pub(crate) async fn init_unpack_progress(
    target_dir: &Path,
    source_file: &Path,
    multi_progress: MultiProgress,
) -> Result<ProgressBar> {
    let progress_bar = multi_progress.add(ProgressBar::new(fs::metadata(source_file).await?.len()));
    progress_bar.set_style(get_progress_style().await);
    progress_bar.set_message(format!(
        "Unpacking {} to {}",
        source_file.display(),
        target_dir.display()
    ));
    Ok(progress_bar)
}

/// Downloads the requested file to the /tmp directory
pub(crate) async fn download_file(
    download: &Download,
    multi_progress: MultiProgress,
) -> Result<PathBuf> {
    let output_dir = download.download_dir()?;

    if files::check_if_exists(&output_dir).await {
        fs::remove_dir_all(&output_dir).await?;
    }

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&output_dir)
        .await
        .with_context(|| {
            format!(
                "[Download] Failed creating destination file: {}",
                output_dir.display()
            )
        })?;

    let download_progress_bar =
        init_download_progress(download, &output_dir, multi_progress.clone()).await;

    downloads::download_to_async_write(
        &download.download_url,
        &mut download_progress_bar.wrap_async_write(file),
    )
    .await?;

    download_progress_bar.set_style(get_message_bar_style().await);
    download_progress_bar.finish_with_message(download_progress_bar.message().replacen(
        "Downloading",
        "Downloaded",
        1,
    ));

    Ok(output_dir)
}

pub(crate) async fn validate_file(
    file_name: &str,
    path: &Path,
    hash: hashing::HashSums,
    multi_progress: MultiProgress,
) -> Result<()> {
    let file = File::open(path)
        .await
        .context("[Hash Check] Failed opening download file for checking. Was the file moved?")?;

    let hash_progress_bar = init_hash_progress(path, multi_progress).await?;

    if !hashing::hash_check_file(
        file_name,
        &mut hash_progress_bar.wrap_async_read(BufReader::new(file)),
        hash,
    )
    .await?
    {
        bail!("{} failed validation", path.display());
    }

    hash_progress_bar.set_style(get_message_bar_style().await);
    hash_progress_bar.finish_with_message(hash_progress_bar.message().replacen(
        "Validating",
        "Validated",
        1,
    ));

    Ok(())
}

/// Downloads the latest wine version for all the apps found
pub async fn run_quick_downloads(force: bool, whats_new: bool) -> Result<Vec<Release>> {
    let found_apps = apps::list_installed_apps().await;
    if found_apps.is_empty() {
        println!("No apps found. Please install at least one app before using this feature.");
        return Err(anyhow!(
            "No apps found. Please install at least one app before using this feature."
        ));
    }
    println!(
        "Found the following apps: {}",
        found_apps
            .iter()
            .map(|app| app.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    );

    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(20));

    // Group apps by their default compatibility tool
    let mut tool_to_apps: std::collections::HashMap<
        String,
        (CompatTool, Vec<apps::AppInstallations>),
    > = std::collections::HashMap::new();

    for app_inst in &found_apps {
        let compat_tool = app_inst.as_app().default_compatibility_tool();
        let tool_name = compat_tool.name.clone();
        tool_to_apps
            .entry(tool_name)
            .or_insert_with(|| (compat_tool, Vec::new()))
            .1
            .push(app_inst.clone());
    }

    let joins = FuturesUnordered::new();
    let mut releases: Vec<Release> = vec![];
    let mut release_and_compat_refs: Vec<(Release, CompatTool)> = vec![];

    for (_, (compat_tool, apps_for_tool)) in tool_to_apps {
        // Get the latest Download info for the compat_tool
        let release = match downloads::list_releases(&compat_tool).await {
            Ok(mut release_list) => release_list.remove(0),
            Err(e) => {
                eprintln!(
                    "Failed to fetch data, make sure you're connected to the internet.\nError: {e}"
                );
                std::process::exit(1)
            }
        };

        // Handle tools with multiple architecture variants
        let download = if compat_tool.has_multiple_asset_variations {
            let variants = release.get_all_download_variants(&compat_tool);
            architecture_variants::select_architecture_variant(&release.tag_name, variants, true)?
        } else {
            release.get_download_info(&compat_tool)
        };

        // Check if any app in the group needs installation
        let mut needs_install = false;
        for app_inst in &apps_for_tool {
            let mut download_path = PathBuf::from(&app_inst.default_install_dir().as_str());
            download_path.push(compat_tool.installation_name(&download.version));
            if !files::check_if_exists(&download_path).await || force {
                needs_install = true;
                break;
            }
        }

        if !needs_install {
            continue;
        }

        // if should show release notes, populate this list with the tools to show
        if whats_new {
            release_and_compat_refs.push((release.clone(), compat_tool.clone()));
        }

        joins.push(download_validate_unpack(
            release.clone(),
            apps_for_tool,
            compat_tool,
            multi_progress.clone(),
        ));

        releases.push(release);
    }

    // Show what is new for each app being downloaded
    if whats_new {
        let mut seen_tags = HashSet::new();

        // filter list to avoid showing the same app tag twice
        let release_and_compat_refs: Vec<(Release, CompatTool)> = release_and_compat_refs
            .into_iter()
            .filter(|(release, _compat)| {
                // We insert a reference to the tag_name into the HashSet
                // so we don't have to allocate/clone Strings!
                seen_tags.insert(release.tag_name.clone())
            })
            .collect();

        for (release, compat_tool) in release_and_compat_refs {
            show_whatsnew(&release, &compat_tool).await;
        }
    }

    joins
        .for_each(|res| {
            if let Err(e) = res {
                multi_progress.println(format!("{e}")).unwrap();
            }
            future::ready(())
        })
        .await;
    multi_progress.clear().unwrap();

    Ok(releases)
}

async fn show_whatsnew(release: &Release, compat_tool: &CompatTool) {
    // Show release notes before downloading if --whats-new was passed
    const WHATS_NEW_LINES: usize = 40;

    println!();
    println!("  ┌{}┐", "─".repeat(50));
    println!("  │ {:^48} │", "Release Notes");
    println!("  └{}┘", "─".repeat(50));

    // if release.body.is_none() || release.body.as_ref().is_some_and(|s| s.is_empty()) {
    //     release.body = Some("\n  (could not fetch release notes)".to_string());
    // }

    let url = format!(
        "{}{}/{}/releases/tag/{}",
        compat_tool.forge.get_user_url(),
        compat_tool.repository_account,
        compat_tool.repository_name,
        release.tag_name
    );
    println!("\n  {}: {}", release.tag_name, url);
    match &release.body {
        Some(body) => {
            let all_lines: Vec<&str> = body.lines().collect();
            let notes = all_lines[..all_lines.len().min(WHATS_NEW_LINES)].join("\n");
            if all_lines.len() > WHATS_NEW_LINES {
                println!("\n{}\n  ⋯ [truncated]", notes);
            } else {
                println!("\n{}", notes);
            }
        }
        None => println!("\n  (no release notes)"),
    }
    println!();
}

/// Start the Download for the selected app
///
/// If no app is provided, the user is prompted for which version of Wine/Proton to use and what directory to extract to
pub async fn download_to_selected_app(app: Option<apps::App>) -> Result<Vec<Release>> {
    // Get the folder to install Wine/Proton into
    let app_inst = match app.clone() {
        // If the user selected an app (Steam/Lutris)...
        Some(app) => match app.detect_installation_method().await {
            installed_apps if installed_apps.is_empty() => {
                println!("Install location for selected app(s) not found. Exiting.");
                std::process::exit(0);
            }

            // Figure out which versions of the App the user has (Native/Flatpak)
            installed_apps if installed_apps.len() == 1 => {
                println!(
                    "Detected {}. Installing to {}",
                    installed_apps[0], installed_apps[0]
                );
                installed_apps[0].clone()
            }
            // If the user has more than one installation method, ask them which one they would like to use
            installed_apps => Select::new(
                "Detected several app versions, which would you like to use?",
                installed_apps,
            )
            .prompt()
            .unwrap_or_else(|_| std::process::exit(0)),
        },
        // If the user didn't select an app, ask them what directory they want to install to
        None => apps::AppInstallations::new_custom_app_install(
            Text::new("Installation path:")
                .with_autocomplete(file_path::FilePathCompleter::default())
                .with_help_message(&format!(
                    "Current directory: {}",
                    &std::env::current_dir()
                        .unwrap_or_else(|_| std::process::exit(0))
                        .to_string_lossy()
                ))
                .with_default(
                    &std::env::current_dir()
                        .unwrap_or_else(|_| std::process::exit(0))
                        .to_string_lossy(),
                )
                .prompt()
                .unwrap_or_else(|_| std::process::exit(0)),
        ),
    };

    // if an app was selected, filter compatible tools
    let available_sources = match app {
        // Use the default for the app
        Some(app) => CompatTool::sources_for_app(&app),
        // Or have the user select which one
        None => CompatTools.clone(),
    };

    // TODO: maybe change to multi-select ?
    let selected_tool = Select::new(
        "Choose the compatibility tool you want to install:",
        available_sources, // variants::ALL_VARIANTS.to_vec(),
    )
    .prompt()
    .unwrap_or_else(|_| std::process::exit(0));

    let releases = {
        let release_list = match downloads::list_releases(&selected_tool).await {
            Ok(data) => data,
            Err(e) => {
                eprintln!(
                    "Failed to fetch data, make sure you're connected to the internet.\nError: {e}"
                );
                std::process::exit(1)
            }
        };

        // Let the user choose which releases they want to use
        stream::iter(
            helper_menus::multiple_select_menu(
                "Select the versions you want to download:",
                release_list,
            )
            .unwrap_or_else(|e| {
                eprintln!("The tag list could not be processed.\nError: {e}");
                vec![]
            }),
        )
        .filter_map(|release| async {
            if should_download(
                &release,
                &mut app_inst
                    .installation_dir(&selected_tool)
                    .unwrap()
                    .join(selected_tool.installation_name(&release.tag_name)),
            )
            .await
            {
                Some(release)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .await
    };

    // let tool = selected_tool.clone();
    // Check if the selected tool has multiple asset variations
    let downloads: Vec<Download> = if selected_tool.has_multiple_asset_variations {
        releases
            .iter()
            .map(|release| {
                let variants = release.get_all_download_variants(&selected_tool);

                architecture_variants::select_architecture_variant(
                    &release.tag_name,
                    variants,
                    false,
                )
                .unwrap_or_else(|_| std::process::exit(1))
            })
            .collect::<Vec<Download>>()
    } else {
        releases
            .iter()
            .map(|release| release.get_download_info(&selected_tool))
            .collect()
    };

    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(20));

    let tasks = downloads.into_iter().map(|download| {
        // let release = release.clone();
        let progress = multi_progress.clone();
        let tool = selected_tool.clone();
        let for_apps = vec![app_inst.clone()];

        // Handle tools with multiple architecture variants
        tokio::spawn(async move {
            download_validate_unpack_with_download(download.clone(), for_apps, tool, progress).await
        })
    });

    for task in tasks
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await
    {
        let err: Option<anyhow::Error> = match task {
            Ok(Ok(())) => None,
            Ok(Err(e)) => Some(e),
            Err(join_err) => Some(anyhow!(join_err)),
        };

        if let Some(e) = err {
            eprintln!("{e}");
            return Err(anyhow!("Installation failed with Error"));
        }
    }

    Ok(releases)
}

async fn download_validate_unpack(
    release: Release,
    for_apps: Vec<apps::AppInstallations>,
    compat_tool: CompatTool,
    multi_progress: MultiProgress,
) -> Result<()> {
    let download = release.get_download_info(&compat_tool);
    download_validate_unpack_with_download(download, for_apps, compat_tool, multi_progress).await
}

async fn download_validate_unpack_with_download(
    download: Download,
    for_apps: Vec<apps::AppInstallations>,
    compat_tool: CompatTool,
    multi_progress: MultiProgress,
) -> Result<()> {
    // Download ONCE
    let file = download_file(&download, multi_progress.clone())
        .await
        .with_context(|| {
            format!(
                "Error downloading {}, make sure you're connected to the internet",
                download.version
            )
        })?;

    // Validate ONCE
    match download.hash_sum {
        Some(ref git_hash_sum) => {
            let hash_content = &downloads::download_file_into_memory(&git_hash_sum.sum_content)
                .await
                .with_context(|| {
                    format!(
                        "Error getting expected download hash for {}",
                        &download.version
                    )
                })?;
            let hash_sum = hashing::HashSums {
                sum_content: hash_content.to_owned(),
                sum_type: git_hash_sum.sum_type.clone(),
            };

            validate_file(&download.file_name, &file, hash_sum, multi_progress.clone()).await?;
        }
        None => {
            println!("No sum files available, skipping");
        }
    }

    // Unpack to EACH app
    for for_app in &for_apps {
        let install_dir = for_app.installation_dir(&compat_tool).unwrap();
        let install_name = compat_tool.installation_name(&download.version);
        let install_path = install_dir.join(&install_name);

        if files::check_if_exists(&install_path).await {
            fs::remove_dir_all(&install_path).await.with_context(|| {
                format!(
                    "Error removing existing install at {}",
                    install_path.display()
                )
            })?;
        }

        let unpack_progress_bar =
            init_unpack_progress(&install_dir.clone(), &file, multi_progress.clone())
                .await
                .with_context(|| format!("Error unpacking {}", file.display()))?;

        let decompressor = files::Decompressor::from_path(&file)
            .await
            .with_context(|| format!("Error checking file type of {}", file.display()))?;

        files::unpack_file(
            &compat_tool,
            &download,
            unpack_progress_bar.wrap_async_read(decompressor),
            &install_dir,
        )
        .await
        .with_context(|| format!("Error unpacking {}", file.display()))?;

        unpack_progress_bar.set_style(get_message_bar_style().await);
        unpack_progress_bar
            .finish_with_message(format!("Done! {} installed to {}", compat_tool, for_app));
    }

    Ok(())
}

/// Checks if the selected Release/version is already installed.
///
/// Will prompt the user to overwrite existing files
async fn should_download(release: &Release, install_dir: &mut PathBuf) -> bool {
    // Check if versions exist in disk.
    // If they do, ask the user if it should be overwritten
    !files::check_if_exists(install_dir).await
        || helper_menus::confirm_menu(
            format!(
                "Version {} exists in the installation path. Overwrite?",
                &release.tag_name
            ),
            String::from("If you choose yes, you will re-install it."),
            false,
        )
}

async fn get_progress_style() -> ProgressStyle {
    PROGRESS_BAR_STYLE.get_or_init(|| {
        future::ready(ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})").unwrap()
            .progress_chars("#>-"))
    }).await.clone()
}

pub(crate) async fn get_message_bar_style() -> ProgressStyle {
    MESSAGE_BAR_STYLE
        .get_or_init(|| future::ready(ProgressStyle::default_bar().template("{msg}").unwrap()))
        .await
        .clone()
}
