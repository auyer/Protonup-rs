use anyhow::{Context, Result, anyhow, bail};
use futures_util::stream::FuturesUnordered;
use futures_util::{StreamExt, future, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use inquire::{Select, Text};
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

use crate::{file_path, helper_menus};

/// Architecture variant for Proton CachyOS
#[derive(Debug, Clone)]
pub struct ArchitectureVariant {
    /// The architecture variant name (x86_64, x86_64_v2, x86_64_v3, x86_64_v4...)
    pub name: String,
    /// Extended description of this variant
    pub description: String,
    /// The download information for this variant
    pub download: Download,
}

impl std::fmt::Display for ArchitectureVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {}", self.name, self.description)
    }
}

/// Extracts the architecture variant from file name
fn get_architecture_variant(file_name: &str) -> u8 {
    // 1: x86_64, 2: x86_64_v2, 3: x86_64_v3, 4: x86_64_v4
    if file_name.contains("_v4") {
        4
    } else if file_name.contains("_v3") {
        3
    } else if file_name.contains("_v2") {
        2
    } else if file_name.contains("-x86_64.") {
        1
    } else {
        0
    }
}

/// Gets the variant name string from the variant code
fn get_variant_name(variant_code: u8) -> &'static str {
    match variant_code {
        1 => "x86_64",
        2 => "x86_64_v2",
        3 => "x86_64_v3",
        4 => "x86_64_v4",
        _ => "unknown",
    }
}

/// Gets an extended description for an architecture variant
fn get_architecture_description(variant_code: u8) -> String {
    match variant_code {
        4 => "Experimental - optimized for AVX-512",
        3 => "Modern CPUs - optimized for AVX2",
        2 => "Recommended - optimized for SSE3",
        1 => "Universal - all x86-64 CPUs",
        _ => "Unknown",
    }
    .to_string()
}

/// Menu for selecting proton cachyos arch, returns selected or _v2 if in quick mode
pub fn select_architecture_variant(
    release_name: &str,
    variants: Vec<Download>,
    quick_mode: bool,
) -> Result<Download> {
    if variants.is_empty() {
        return Err(anyhow!("No architecture variants available"));
    }

    if quick_mode {
        let default = variants
            .iter()
            .find(|d| d.file_name.contains("_v2"))
            .or_else(|| variants.first())
            .unwrap();
        println!(
            "Selected {} by default",
            get_variant_name(get_architecture_variant(&default.file_name))
        );
        return Ok(default.clone());
    }

    // Create ArchitectureVariant objects with descriptions
    let arch_variants: Vec<ArchitectureVariant> = variants
        .iter()
        .filter_map(|download| {
            let variant_code = get_architecture_variant(&download.file_name);
            if variant_code == 0 {
                return None;
            }
            let variant_name = get_variant_name(variant_code).to_string();
            let description = get_architecture_description(variant_code);
            Some(ArchitectureVariant {
                name: variant_name,
                description,
                download: download.clone(),
            })
        })
        .collect();

    if arch_variants.is_empty() {
        return Ok(variants.into_iter().next().unwrap()); // fallback to first if none found
    }

    // Sort variants
    let mut sorted_variants = arch_variants;
    sorted_variants.sort_by(|a, b| {
        let order = |name: &str| -> u8 {
            match name {
                "x86_64" => 1,
                "x86_64_v2" => 2,
                "x86_64_v3" => 3,
                "x86_64_v4" => 4,
                _ => 99,
            }
        };
        order(&a.name).cmp(&order(&b.name))
    });

    let selected = Select::new(
        format!("Select CPU architecture for release '{}' :", release_name).as_str(),
        sorted_variants,
    )
    .prompt()
    .unwrap_or_else(|_| std::process::exit(0));

    Ok(selected.download)
}

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
                "[Download] Failed creating destination file : {}",
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
pub async fn run_quick_downloads(force: bool) -> Result<Vec<Release>> {
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

    let joins = FuturesUnordered::new();
    let mut releases: Vec<Release> = vec![];
    for app_inst in found_apps {
        let compat_tool = app_inst.as_app().default_compatibility_tool();

        // Get the latest Download info for the compat_tool
        let release = match downloads::list_releases(&compat_tool).await {
            // Get the Download info from the first item on the list, the latest version
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
            let variants = release.get_all_download_variants(&app_inst, &compat_tool);
            select_architecture_variant(&release.tag_name, variants, true)?
        } else {
            release.get_download_info(&app_inst, &compat_tool)
        };

        let mut download_path = PathBuf::from(&app_inst.default_install_dir().as_str());
        download_path.push(compat_tool.installation_name(&download.version));
        if files::check_if_exists(&download_path.clone()).await && !force {
            continue;
        }

        joins.push(download_validate_unpack(
            release.clone(),
            app_inst,
            compat_tool,
            multi_progress.clone(),
        ));

        releases.push(release);
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
                "Select the versions you want to download :",
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
                let variants = release.get_all_download_variants(&app_inst, &selected_tool);

                select_architecture_variant(&release.tag_name, variants, false)
                    .unwrap_or_else(|_| std::process::exit(1))
            })
            .collect::<Vec<Download>>()
    } else {
        releases
            .iter()
            .map(|release| release.get_download_info(&app_inst, &selected_tool))
            .collect()
    };

    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(20));

    let tasks = downloads.into_iter().map(|download| {
        // let release = release.clone();
        let progress = multi_progress.clone();
        let tool = selected_tool.clone();
        let app_inst = app_inst.clone();

        // Handle tools with multiple architecture variants
        tokio::spawn(async move {
            download_validate_unpack_with_download(download.clone(), app_inst, tool, progress).await
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
    for_app: apps::AppInstallations,
    compat_tool: CompatTool,
    multi_progress: MultiProgress,
) -> Result<()> {
    let download = release.get_download_info(&for_app, &compat_tool);
    download_validate_unpack_with_download(download, for_app, compat_tool, multi_progress).await
}

async fn download_validate_unpack_with_download(
    download: Download,
    for_app: apps::AppInstallations,
    compat_tool: CompatTool,
    multi_progress: MultiProgress,
) -> Result<()> {
    let install_dir = for_app.installation_dir(&compat_tool).unwrap();
    let file = download_file(&download, multi_progress.clone())
        .await
        .with_context(|| {
            format!(
                "Error downloading {}, make sure you're connected to the internet",
                download.version
            )
        })?;
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

    let install_name = compat_tool.installation_name(&download.version);
    let install_path = install_dir.join(install_name.clone());
    if files::check_if_exists(&install_path).await {
        fs::remove_dir_all(&install_path).await.with_context(|| {
            format!(
                "Error removing existing install at {}",
                install_path.display()
            )
        })?;
    }

    let unpack_progress_bar = init_unpack_progress(&install_dir.clone(), &file, multi_progress)
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
    unpack_progress_bar.finish_with_message(format!(
        "Done! {} installed in {}/{}\nYour app might require a restart to detect {}",
        compat_tool,
        download
            .for_app
            .installation_dir(&compat_tool)
            .unwrap()
            .to_str()
            .unwrap(),
        install_name,
        compat_tool
    ));
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

async fn get_message_bar_style() -> ProgressStyle {
    MESSAGE_BAR_STYLE
        .get_or_init(|| future::ready(ProgressStyle::default_bar().template("{msg}").unwrap()))
        .await
        .clone()
}
