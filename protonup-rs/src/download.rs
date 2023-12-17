use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use inquire::{Select, Text};

use std::fs;
use std::path::{Path, PathBuf};
use std::{
    sync::{atomic::Ordering, Arc},
    thread,
    time::Duration,
};

use crate::{file_path, helper_menus};

use libprotonup::{
    apps, constants, files,
    github::{self, Download, Release},
    utils,
    variants::{self, Variant},
};

pub(crate) async fn download_file(download: Download) -> Result<PathBuf, String> {
    let mut temp_dir = utils::expand_tilde(constants::TEMP_DIR).unwrap();

    temp_dir.push(if download.download_url.ends_with("tar.gz") {
        format!("{}.tar.gz", &download.version)
    } else if download.download_url.ends_with("tar.xz") {
        format!("{}.tar.xz", &download.version)
    } else {
        eprintln!("Downloaded file wasn't of the expected type. (tar.(gz/xz)");
        std::process::exit(1)
    });

    let git_hash = files::download_file_into_memory(&download.sha512sum_url)
        .await
        .unwrap();

    if temp_dir.exists() {
        fs::remove_file(&temp_dir).unwrap();
    }

    let (progress, done) = files::create_progress_trackers();
    let progress_read = Arc::clone(&progress);
    let done_read = Arc::clone(&done);
    let url = String::from(&download.download_url);
    let tmp_dir = String::from(temp_dir.to_str().unwrap());

    // start ProgressBar in another thread
    thread::spawn(move || {
        let pb = ProgressBar::with_draw_target(
            Some(download.size),
            ProgressDrawTarget::stderr_with_hz(20),
        );
        pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})").unwrap()
        .progress_chars("#>-"));
        pb.set_message(format!("Downloading {}", url.split('/').last().unwrap()));
        let wait_time = Duration::from_millis(50); // 50ms wait is about 20Hz
        loop {
            let newpos = progress_read.load(Ordering::Relaxed);
            pb.set_position(newpos as u64);
            if done_read.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(wait_time);
        }
        pb.set_message(format!("Downloaded {url} to {tmp_dir}"));
        pb.abandon(); // closes progress bar without blanking terminal

        println!("Checking file integrity"); // This is being printed here because the progress bar needs to be closed before printing.
    });

    files::download_file_progress(
        download.download_url,
        download.size,
        temp_dir.clone().as_path(),
        progress,
        done,
    )
    .await
    .unwrap();

    if !files::hash_check_file(temp_dir.to_str().unwrap().to_string(), git_hash).unwrap() {
        return Err("Failed checking file hash".to_string());
    }

    Ok(temp_dir)
}

pub(crate) async fn unpack_file(
    dowaload_path: &Path,
    install_path: &str,
    wine_version: &Variant,
) -> Result<(), String> {
    let install_dir = utils::expand_tilde(install_path).unwrap();

    fs::create_dir_all(&install_dir).unwrap();

    println!("Unpacking files into install location. Please wait");
    files::decompress(dowaload_path, install_dir.as_path()).unwrap();
    println!(
        "Done! Restart {}. {} installed in {}",
        wine_version.intended_application(),
        wine_version,
        install_dir.to_string_lossy(),
    );
    Ok(())
}

pub async fn run_quick_downloads() {
    let found_apps = apps::list_installed_apps();
    if found_apps.is_empty() {
        println!("No apps found. Please install at least one app before using this feature.");
        return;
    }
    println!(
        "Found the following apps: {}",
        found_apps
            .iter()
            .map(|app| app.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    );

    for app_inst in &found_apps {
        let wine_version = app_inst.into_app().app_wine_version();
        let destination = app_inst.default_install_dir().to_string();
        println!(
            "\nQuick Download: {} for {} into -> {}",
            wine_version,
            app_inst.into_app(),
            destination
        );

        // Get the latest Download info for the wine_version
        let download = match github::list_releases(&wine_version.get_github_parameters()).await {
            // Get the Download info from the first item on the list, the latest version
            Ok(release_list) => release_list[0].get_download_info(),
            Err(e) => {
                eprintln!("Failed to fetch Github data, make sure you're connected to the internet.\nError: {}", e);
                std::process::exit(1)
            }
        };

        let file = download_file(download).await.unwrap();
        unpack_file(&file, &destination, &wine_version)
            .await
            .unwrap_or_else(|e| {
                eprintln!(
                    "Failed unpacking file {} into {}. Error: {}",
                    file.to_string_lossy(),
                    destination,
                    e
                );
            });
    }
}

/// Start the Download for the selected app
/// If no app is provided, the user is prompted for which version of Wine/Proton to use and what directory to extract to
pub async fn download_to_selected_app(app: Option<apps::App>) {
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
    let install_dir: String = match app {
        // If the user selected an app (Steam/Lutris)...
        Some(app) => match app.detect_installation_method() {
            installed_apps if installed_apps.len() == 0 => {
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
                installed_apps[0].default_install_dir().to_string()
            }
            // If the user has more than one installation method, ask them which one they would like to use
            installed_apps => Select::new(
                "Detected several app versions, which would you like to use?",
                installed_apps,
            )
            .prompt()
            .unwrap_or_else(|_| std::process::exit(0))
            .default_install_dir()
            .to_string(),
        },
        // If the user didn't select an app, ask them what directory they want to install to
        None => Text::new("Installation path:")
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
    };

    let release_list = match github::list_releases(&wine_version.get_github_parameters()).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to fetch Github data, make sure you're connected to the internet.\nError: {}", e);
            std::process::exit(1)
        }
    };

    // versions_to_install = vec![];

    // Let the user choose which releases they want to use
    let mut release_list = match helper_menus::multiple_select_menu(
        "Select the versions you want to download :",
        release_list,
    ) {
        Ok(release_list) => release_list,
        Err(e) => {
            eprintln!("The tag list could not be processed.\nError: {}", e);
            vec![]
        }
    };

    // Check if the versions the user selected are already on the disk
    check_if_already_downloaded(&mut release_list, &install_dir).await;

    // Prepare the download for the user's chosen releases/versions
    // TODO Look into using async in a way to download multiple files at once, would need to .join all the download_file() 'Futures'
    for release in &release_list {
        match download_file(release.get_download_info()).await {
            Ok(file) => {
                // TODO: should just upack once and copy to all folders
                unpack_file(&file, &install_dir, &wine_version)
                    .await
                    .unwrap();
            }
            Err(e) => {
                eprintln!(
                    "Error downloading {}, make sure you're connected to the internet\nError: {}",
                    release.tag_name, e
                )
            }
        }
    }
}

/// Checks if the selected Release/version is already installed.
/// Will prompt the user to overwrite existing files
async fn check_if_already_downloaded(release_list: &mut Vec<Release>, install_dir: &str) {
    release_list.retain(|release| {
        // Check if versions exist in disk.
        // If they do, ask the user if it should be overwritten
        !(files::check_if_exists(&install_dir, &release.tag_name)
            && !helper_menus::confirm_menu(
                format!(
                    "Version {} exists in the installation path. Overwrite?",
                    &release.tag_name
                ),
                String::from("If you choose yes, you will re-install it."),
                false,
            ))
    });
}
