use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use arcstr::ArcStr;
use futures_util::stream::FuturesUnordered;
use futures_util::{future, stream, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use inquire::{Select, Text};
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, BufReader};
use tokio::sync::OnceCell;

use libprotonup::files::Decompressor;
use libprotonup::{
    apps, files,
    github::{self, Download, Release},
    utils,
    variants::{self, Variant},
};

use crate::{file_path, helper_menus};

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
        download.download_url.split('/').last().unwrap(),
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
    target_dir: ArcStr,
    source_file: &Path,
    multi_progress: MultiProgress,
) -> Result<ProgressBar> {
    let progress_bar = multi_progress.add(ProgressBar::new(fs::metadata(source_file).await?.len()));
    progress_bar.set_style(get_progress_style().await);
    progress_bar.set_message(format!(
        "Unpacking {} to {}",
        source_file.display(),
        target_dir
    ));
    Ok(progress_bar)
}

pub(crate) async fn get_expected_hash(download: &Download) -> Result<String> {
    files::download_file_into_memory(&download.sha512sum_url).await
}

/// Downloads the requested file to the /tmp directory
pub(crate) async fn download_file(
    download: &Download,
    multi_progress: MultiProgress,
    mut output_dir: PathBuf,
) -> Result<PathBuf> {
    output_dir.push(if download.download_url.ends_with("tar.gz") {
        format!("{}.tar.gz", &download.version)
    } else if download.download_url.ends_with("tar.xz") {
        format!("{}.tar.xz", &download.version)
    } else {
        return Err(anyhow!(
            "Downloaded file wasn't of the expected type. (tar.(gz/xz)"
        ));
    });

    if files::check_if_exists(&output_dir.to_string_lossy(), "").await {
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

    files::download_to_async_write(
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
    path: &Path,
    hash: &str,
    multi_progress: MultiProgress,
) -> Result<()> {
    let file = File::open(path)
        .await
        .context("[Hash Check] Failed opening download file for checking. Was the file moved?")?;

    let hash_progress_bar = init_hash_progress(path, multi_progress).await?;

    if !files::hash_check_file(
        &mut hash_progress_bar.wrap_async_read(BufReader::new(file)),
        hash,
    )
    .await?
    {
        return Err(anyhow::Error::msg(format!(
            "{} failed validation",
            path.display()
        )));
    }

    hash_progress_bar.set_style(get_message_bar_style().await);
    hash_progress_bar.finish_with_message(hash_progress_bar.message().replacen(
        "Validating",
        "Validated",
        1,
    ));

    Ok(())
}

/// Prepares downloaded file to be decompressed
///
/// Parses the passed in data and ensures the destination directory is created
pub(crate) async fn unpack_file<R: AsyncRead + Unpin>(reader: R, install_path: &str) -> Result<()> {
    let install_dir = utils::expand_tilde(install_path).unwrap();

    fs::create_dir_all(&install_dir).await.unwrap();

    files::decompress(reader, install_dir.as_path())
        .await
        .unwrap();

    Ok(())
}

/// Downloads the latest wine version for all the apps found
pub async fn run_quick_downloads(force: bool) -> Result<Vec<Release>> {
    let found_apps = apps::list_installed_apps().await;
    if found_apps.is_empty() {
        println!("No apps found. Please install at least one app before using this feature.");
        return Err(anyhow!("No apps found. Please install at least one app before using this feature."));
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
    for app_inst in &found_apps {
        let wine_version = app_inst.as_app().app_wine_version();
        let destination = app_inst.default_install_dir().to_string();

        // Get the latest Download info for the wine_version
        let release = match github::list_releases(&wine_version.get_github_parameters()).await {
            // Get the Download info from the first item on the list, the latest version
            Ok(mut release_list) => release_list.remove(0),
            Err(e) => {
                eprintln!("Failed to fetch Github data, make sure you're connected to the internet.\nError: {}", e);
                std::process::exit(1)
            }
        };
        let download = release.get_download_info();

        if files::check_if_exists(
            &app_inst.default_install_dir(),
            download.output_dir(&wine_version),
        )
        .await
            && !force
        {
            continue;
        }

        let output_dir = tempfile::tempdir().expect("Failed to create tempdir");

        joins.push(download_validate_unpack(
            release.clone(),
            ArcStr::from(destination),
            wine_version,
            output_dir.into_path(),
            multi_progress.clone(),
        ));

        releases.push(release);
    }

    joins
        .for_each(|res| {
            if let Err(e) = res {
                multi_progress.println(format!("{}", e)).unwrap();
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
    // Get the version of Wine/Proton to install
    let wine_version = match app {
        // Use the default for the app
        Some(app) => app.app_wine_version(),
        // Or have the user select which one
        None => Select::new(
            "Choose the variant you want to install:",
            variants::ALL_VARIANTS.to_vec(),
        )
        .prompt()
        .unwrap_or_else(|_| std::process::exit(0)),
    };

    // Get the folder to install Wine/Proton into
    let install_dir: ArcStr = match app {
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
                    installed_apps[0],
                    installed_apps[0].default_install_dir()
                );
                installed_apps[0].default_install_dir()
            }
            // If the user has more than one installation method, ask them which one they would like to use
            installed_apps => Select::new(
                "Detected several app versions, which would you like to use?",
                installed_apps,
            )
            .prompt()
            .unwrap_or_else(|_| std::process::exit(0))
            .default_install_dir(),
        },
        // If the user didn't select an app, ask them what directory they want to install to
        None => ArcStr::from(
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

    let releases = {
        let release_list = match github::list_releases(&wine_version.get_github_parameters()).await
        {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to fetch Github data, make sure you're connected to the internet.\nError: {}", e);
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
                eprintln!("The tag list could not be processed.\nError: {}", e);
                vec![]
            }),
        )
        .filter_map(|r| async {
            if should_download(&r, install_dir.clone()).await {
                Some(r)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .await
    };

    let multi_progress = MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(20));

    stream::iter(releases.clone())
        .map(|r| {
            let dir = install_dir.clone();
            let temp_dir = tempfile::tempdir()
                .expect("Unable to create tempdir")
                .into_path();
            let progress = multi_progress.clone();
            tokio::spawn(async move {
                download_validate_unpack(r, dir, wine_version, temp_dir, progress).await
            })
        })
        .collect::<FuturesUnordered<_>>()
        .await
        .for_each(|res| {
            if let Err(e) = res {
                multi_progress.println(format!("{}", e)).unwrap();
            }
            future::ready(())
        })
        .await;

    Ok(releases)
}

async fn download_validate_unpack(
    release: Release,
    install_dir: ArcStr,
    wine_version: Variant,
    temp_dir: PathBuf,
    multi_progress: MultiProgress,
) -> Result<()> {
    let download = release.get_download_info();
    let file = download_file(&download, multi_progress.clone(), temp_dir)
        .await
        .with_context(|| {
            format!(
                "Error downloading {}, make sure you're connected to the internet",
                release.tag_name
            )
        })?;

    validate_file(
        &file,
        &get_expected_hash(&download).await.with_context(|| {
            format!(
                "Error getting expected download hash for {}",
                &release.tag_name
            )
        })?,
        multi_progress.clone(),
    )
    .await?;

    let download = release.get_download_info();
    let output_dir = download.output_dir(&wine_version);
    if files::check_if_exists(&install_dir, output_dir).await {
        let path = Path::new(&install_dir.as_str()).join(output_dir);
        fs::remove_dir_all(&path)
            .await
            .with_context(|| format!("Error removing existing install at {}", path.display()))?;
    }

    let unpack_progress_bar = init_unpack_progress(install_dir.clone(), &file, multi_progress)
        .await
        .with_context(|| format!("Error unpacking {}", file.display()))?;

    let decompressor = Decompressor::from_path(&file)
        .await
        .with_context(|| format!("Error checking file type of {}", file.display()))?;
    unpack_file(
        unpack_progress_bar.wrap_async_read(decompressor),
        &install_dir,
    )
    .await
    .with_context(|| format!("Error unpacking {}", file.display()))?;

    unpack_progress_bar.set_style(get_message_bar_style().await);
    unpack_progress_bar.finish_with_message(format!(
        "Done! Restart {}. {} installed in {}",
        wine_version.intended_application(),
        wine_version,
        install_dir.to_string()
    ));
    Ok(())
}

/// Checks if the selected Release/version is already installed.
///
/// Will prompt the user to overwrite existing files
async fn should_download(release: &Release, install_dir: ArcStr) -> bool {
    // Check if versions exist in disk.
    // If they do, ask the user if it should be overwritten
    !files::check_if_exists(&install_dir, &release.tag_name).await
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
