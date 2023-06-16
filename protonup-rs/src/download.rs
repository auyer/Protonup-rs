use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

use std::fs;
use std::path::{Path, PathBuf};
use std::{
    sync::{atomic::Ordering, Arc},
    thread,
    time::Duration,
};

use libprotonup::{constants, files, github, utils, variants};

pub(crate) async fn download_file(
    tag: &str,
    source: &variants::VariantParameters,
) -> Result<PathBuf, String> {
    let mut temp_dir = utils::expand_tilde(constants::TEMP_DIR).unwrap();

    let download = match github::fetch_data_from_tag(tag, source).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to fetch GitHub data, make sure you're connected to the internet\nError: {}", e);
            std::process::exit(1)
        }
    };

    temp_dir.push(if download.download.ends_with("tar.gz") {
        format!("{}.tar.gz", &download.version)
    } else if download.download.ends_with("tar.xz") {
        format!("{}.tar.xz", &download.version)
    } else {
        eprintln!("Downloaded file wasn't of the expected type. (tar.(gz/xz)");
        std::process::exit(1)
    });

    let git_hash = files::download_file_into_memory(&download.sha512sum)
        .await
        .unwrap();

    if temp_dir.exists() {
        fs::remove_file(&temp_dir).unwrap();
    }

    let (progress, done) = files::create_progress_trackers();
    let progress_read = Arc::clone(&progress);
    let done_read = Arc::clone(&done);
    let url = String::from(&download.download);
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
        download.download,
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
    source: &variants::VariantParameters,
) -> Result<(), String> {
    let install_dir = utils::expand_tilde(install_path).unwrap();

    fs::create_dir_all(&install_dir).unwrap();

    println!("Unpacking files into install location. Please wait");
    files::decompress(dowaload_path, install_dir.as_path()).unwrap();
    let source_type = source.variant_type();
    println!(
        "Done! Restart {}. {} installed in {}",
        source_type.intended_application(),
        source_type,
        install_dir.to_string_lossy(),
    );
    Ok(())
}
