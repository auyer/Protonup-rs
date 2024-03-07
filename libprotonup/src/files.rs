use super::constants;
use crate::utils;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use reqwest::header::USER_AGENT;
use sha2::{Digest, Sha512};
use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tar::Archive;
use xz2::read::XzDecoder;

/// decompress will detect the extension and decompress the file with the appropriate function
pub fn decompress(from_path: &Path, destination_path: &Path) -> Result<()> {
    let path_str = from_path.as_os_str().to_string_lossy();

    if path_str.ends_with("tar.gz") {
        decompress_gz(from_path, destination_path)
    } else if path_str.ends_with("tar.xz") {
        decompress_xz(from_path, destination_path)
    } else {
        println!("no decompress\nPath: {:?}", from_path);
        Ok(())
    }
}

/// Decompress a tar.gz file
fn decompress_gz(from_path: &Path, destination_path: &Path) -> Result<()> {
    let file = File::open(from_path).with_context(|| {
        format!(
            "[Decompressing] Failed to open file from Path: {}",
            from_path.display(),
        )
    })?;

    let mut archive = Archive::new(GzDecoder::new(file));

    archive.unpack(destination_path).with_context(|| {
        format!(
            "[Decompressing] Failed to unpack into destination : {}",
            destination_path.display()
        )
    })?;
    Ok(())
}

/// Decompress a tar.xz file
fn decompress_xz(from_path: &Path, destination_path: &Path) -> Result<()> {
    let file = File::open(from_path).with_context(|| {
        format!(
            "[Decompressing] Failed to open file from Path: {}",
            from_path.display()
        )
    })?;

    let mut archive = Archive::new(XzDecoder::new(file));

    archive.unpack(destination_path).with_context(|| {
        format!(
            "[Decompressing] Failed to unpack into destination : {}",
            destination_path.display()
        )
    })?;
    Ok(())
}

/// Creates the progress trackers variable pointers
pub fn create_progress_trackers() -> (Arc<AtomicUsize>, Arc<AtomicBool>) {
    (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    )
}

/// check_if_exists checks if a folder exists in a path
pub fn check_if_exists(path: &str, tag: &str) -> bool {
    let f_path = utils::expand_tilde(format!("{path}{tag}/")).unwrap();
    let p = f_path.as_path();
    p.is_dir()
}

/// list_folders_in_path returns a vector of strings of the folders in a path
pub fn list_folders_in_path(path: &str) -> Result<Vec<String>, anyhow::Error> {
    let f_path = utils::expand_tilde(path).unwrap();
    let p = f_path.as_path();
    let paths: Vec<String> = p
        .read_dir()
        .with_context(|| format!("Failed to read directory : {}", p.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let path = e.path();
            let name = path.file_name().unwrap();
            name.to_str().unwrap().to_string()
        })
        .collect();
    Ok(paths)
}

/// Removes a directory and all its contents
pub fn remove_dir_all(path: &str) -> Result<()> {
    let f_path = utils::expand_tilde(path).unwrap();
    let p = f_path.as_path();
    std::fs::remove_dir_all(p)
        .with_context(|| format!("[Remove] Failed to remove directory : {}", p.display()))?;
    Ok(())
}

/// requires pointers to store the progress, and another to store "done" status
/// Create them with `create_progress_trackers`
pub async fn download_file_progress(
    url: String,
    total_size: u64,
    install_dir: &Path,
    progress: Arc<AtomicUsize>,
    done: Arc<AtomicBool>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header(USER_AGENT, format!("protonup-rs {}", constants::VERSION))
        .send()
        .await
        .with_context(|| format!("[Download] Failed to call remote server on URL : {}", &url))?;

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(install_dir)
        .with_context(|| {
            format!(
                "[Download] Failed creating destination file : {}",
                install_dir.display()
            )
        })?;

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.context("[Download] Failed reading stream from network")?;

        file.write_all(&chunk).with_context(|| {
            format!(
                "[Download] Failed creating destination file : {}",
                install_dir.display()
            )
        })?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        let val = Arc::clone(&progress);
        val.swap(new as usize, Ordering::SeqCst);
    }
    done.swap(true, Ordering::SeqCst);
    Ok(())
}

/// Downloads and returns the text response
pub async fn download_file_into_memory(url: &String) -> Result<String> {
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, format!("protonup-rs {}", constants::VERSION))
        .send()
        .await
        .with_context(|| {
            format!(
                "[Download SHA] Failed to call remote server on URL : {}",
                &url
            )
        })?;

    res.text()
        .await
        .with_context(|| format!("[Download SHA] Failed to read response from URL : {}", &url))
}

/// Checks the downloaded file integrity with the sha512sum
pub fn hash_check_file(file_dir: String, git_hash: String) -> Result<bool> {
    let mut file = File::open(file_dir)
        .context("[Hash Check] Failed opening download file for checking. Was the file moved?")?;
    let mut hasher = Sha512::new();
    io::copy(&mut file, &mut hasher)
        .context("[Hash Check] Failed reading download file for checking")?;

    let hash = hasher.finalize();

    let (git_hash, _) = git_hash
        .rsplit_once(' ')
        .context("[Hash Check] Failed decoding hash file. Is this the right hash ? Please file an issue to protonup-rs !")?;

    if hex::encode(hash) != git_hash.trim() {
        return Ok(false);
    }
    Ok(true)
}
