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
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tar::Archive;

fn path_result(path: &PathBuf) -> String {
    let spath = path.to_str();
    match spath {
        Some(p) => return String::from(p),
        None => return String::from("path missing!"),
    }
}

pub fn decompress(from_path: PathBuf, destination_path: PathBuf) -> Result<()> {
    let file = File::open(&from_path).with_context(|| {
        format!(
            "[Decompressing] Failed to open file from Path: {}",
            path_result(&from_path),
        )
    })?;

    let mut archive = Archive::new(GzDecoder::new(file));

    archive.unpack(&destination_path).with_context(|| {
        format!(
            "[Decompressing] Failed to unpack into destination : {}",
            path_result(&destination_path)
        )
    })?;
    Ok(())
}

/// Creates the progress trackers variable pointers
pub fn create_progress_trackers() -> (Arc<AtomicUsize>, Arc<AtomicBool>) {
    return (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
}

pub fn check_if_exists(path: String, tag: String) -> bool {
    let f_path = utils::expand_tilde(format!("{}/{}", path, tag)).unwrap();
    let p = std::path::Path::new(&f_path);
    p.is_dir()
}

/// requires pointers to store the progress, and another to store "done" status
/// Create them with `create_progress_trackers`
pub async fn download_file_progress(
    url: String,
    total_size: u64,
    install_dir: PathBuf,
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
        .open(&install_dir)
        .with_context(|| {
            format!(
                "[Download] Failed creating destination file : {}",
                path_result(&install_dir)
            )
        })?;

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.context("[Download] Failed reading stream from network")?;

        file.write_all(&chunk).with_context(|| {
            format!(
                "[Download] Failed creating destination file : {}",
                path_result(&install_dir)
            )
        })?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        let val = Arc::clone(&progress);
        val.swap(new as usize, Ordering::SeqCst);
    }
    done.swap(true, Ordering::SeqCst);
    return Ok(());
}

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

    Ok(res
        .text()
        .await
        .with_context(|| format!("[Download SHA] Failed to read response from URL : {}", &url))?)
}

pub fn hash_check_file(file_dir: String, git_hash: String) -> Result<bool> {
    let mut file = File::open(&file_dir)
        .context("[Hash Check] Failed oppening download file for checking. Was the file moved?")?;
    let mut hasher = Sha512::new();
    io::copy(&mut file, &mut hasher)
        .context("[Hash Check] Failed reading download file for checking")?;

    let hash = hasher.finalize();

    let (git_hash, _) = git_hash
        .rsplit_once(" ")
        .context("[Hash Check] Failed decoding hash file. Is this the right hash ? Please file an issue to protonup-rs !")?;

    if hex::encode(&hash) != git_hash.trim() {
        return Ok(false);
    }
    return Ok(true);
}
