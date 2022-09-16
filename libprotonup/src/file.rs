use super::constants;
use crossbeam_channel;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use reqwest::header::USER_AGENT;
use sha2::{Digest, Sha512};
use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::Write;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use tar::Archive;

pub fn decompress(from_path: PathBuf, destination_path: PathBuf) -> Result<(), io::Error> {
    let file = File::open(&from_path)
        .or_else(|err| {
            Err(format!(
                "Failed to open file '{}'. Error : {:?}",
                from_path.to_str().unwrap(),
                err
            ))
        })
        .unwrap();
    let mut archive = Archive::new(GzDecoder::new(file));

    let res = archive.unpack(destination_path);
    match res {
        Ok(_) => return Ok(()),
        Err(_) => {
            return Err(Error::new(
                ErrorKind::Other,
                "Failed to unpack file".to_string(),
            ))
        }
    }
}

pub async fn download_file_progress(
    url: String,
    total_size: u64,
    install_dir: PathBuf,
    position_updater: crossbeam_channel::Sender<u64>,
    message_updater: crossbeam_channel::Sender<String>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header(USER_AGENT, format!("protonup-rs {}", constants::VERSION))
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&install_dir)
        .or_else(|err| {
            Err(format!(
                "Failed to create file '{}'. Error : {:?}",
                install_dir.to_str().unwrap(),
                err
            ))
        })?;

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write_all(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        position_updater.send(new).unwrap();
    }
    drop(position_updater); // drop channel when done sending

    message_updater
        .send(format!(
            "Downloaded {} to {}",
            url,
            install_dir.to_str().unwrap()
        ))
        .unwrap();
    drop(message_updater);
    return Ok(());
}

pub async fn download_file_into_memory(url: &String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, format!("protonup-rs {}", constants::VERSION))
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    res.text()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))
}

pub fn hash_check_file(file_dir: String, git_hash: String) -> bool {
    let mut file = File::open(&file_dir).unwrap();
    let mut hasher = Sha512::new();
    io::copy(&mut file, &mut hasher).unwrap();
    let hash = hasher.finalize();

    let (git_hash, _) = git_hash.rsplit_once(" ").unwrap();
    if hex::encode(&hash) != git_hash.trim() {
        return false;
    }
    return true;
}
