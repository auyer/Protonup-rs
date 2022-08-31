use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::USER_AGENT;
use sha2::{Digest, Sha512};
use std::cmp::min;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io;

use std::io::Write;
use std::path::Path;

use futures_util::StreamExt;

const GITHUB_ACCOUNT: &str = "GloriousEggroll";
const GITHUB_REPO: &str = "proton-ge-custom";
const CONFIG_FILE: &str = "~/.config/protonup/config.ini";
const DEFAULT_INSTALL_DIR: &str = "~/.steam/root/compatibilitytools.d/";
const TEMP_DIR: &str = "/tmp/";

pub async fn list_releases() -> Result<Vec<octocrab::models::repos::Release>, octocrab::Error> {
    let releases = octocrab::instance()
        .repos(GITHUB_ACCOUNT, GITHUB_REPO)
        .releases()
        .list()
        .per_page(10)
        .page(1u32)
        .send()
        // .get_latest()
        .await?
        .take_items();
    Ok(releases)
}

extern crate dirs; // 1.0.4

use std::path::PathBuf;

fn expand_tilde<P: AsRef<Path>>(path_user_input: P) -> Option<PathBuf> {
    let p = path_user_input.as_ref();
    if !p.starts_with("~") {
        return Some(p.to_path_buf());
    }
    if p == Path::new("~") {
        return dirs::home_dir();
    }
    dirs::home_dir().map(|mut h| {
        if h == Path::new("/") {
            // Corner case: `h` root directory;
            // don't prepend extra `/`, just drop the tilde.
            p.strip_prefix("~").unwrap().to_path_buf()
        } else {
            h.push(p.strip_prefix("~/").unwrap());
            h
        }
    })
}

fn install_directory() -> Option<PathBuf> {
    let config_parth = expand_tilde(CONFIG_FILE).unwrap();
    if config_parth.exists() {}
    // println!("{}", config_parth.exists());

    // os.path.exists(CONFIG_FILE):
    //     config.read(CONFIG_FILE)
    //     if config.has_option('protonup', 'installdir'):
    //         return os.path.expanduser(config['protonup']['installdir'])
    expand_tilde(DEFAULT_INSTALL_DIR)
}

#[derive(Default, Debug, PartialEq)]
struct Download {
    version: String,
    date: String,
    sha512sum: String,
    download: String,
    size: u64,
}
// use str::ends_with;
use flate2::read::GzDecoder;
use tar::Archive;

async fn fetch_data(tag: &str) -> Result<Download, octocrab::Error> {
    let mut download = Download::default();
    let release = match tag {
        "latest" => {
            octocrab::instance()
                .repos(GITHUB_ACCOUNT, GITHUB_REPO)
                .releases()
                .get_latest()
                .await?
        }
        _ => {
            octocrab::instance()
                .repos(GITHUB_ACCOUNT, GITHUB_REPO)
                .releases()
                .get_by_tag(tag)
                .await?
        }
    };

    download.version = release.tag_name;
    // download.date = release.published_at;
    for ass in &release.assets {
        if ass.name.ends_with("sha512sum") {
            download.sha512sum = ass.browser_download_url.as_str().to_string();
        }
        if ass.name.ends_with("tar.gz") {
            download.download = ass.browser_download_url.as_str().to_string();
            download.size = ass.size as u64;
        }
    }
    Ok(download)
}
use std::fs;
use std::str;
pub async fn download_file(tag: &str) -> Result<(), String> {
    let mut install_dir = install_directory().unwrap();
    let mut temp_dir = expand_tilde(TEMP_DIR).unwrap();

    let download = fetch_data(tag).await.unwrap();

    let mut sha_temp_dir = temp_dir.clone();
    sha_temp_dir.push("proton.sha512sum");

    temp_dir.push(format!("{}.tar.gz", &download.version));

    // install_dir
    create_dir_all(&install_dir).unwrap();

    let git_hash = download_file_into_memory(&download.sha512sum)
        .await
        .unwrap();

    if temp_dir.exists() {
        fs::remove_file(&temp_dir);
    }

    download_file_progress(&download.download, &download.size, &temp_dir)
        .await
        .unwrap();

    let mut file = File::open(&temp_dir).unwrap();
    let mut hasher = Sha512::new();
    io::copy(&mut file, &mut hasher).unwrap();
    let hash = hasher.finalize();

    let (git_hash, _) = git_hash.rsplit_once(" ").unwrap();
    if hex::encode(&hash) != git_hash.trim() {
        println!("failed validating file with SHA512");
        return Err("failed validating file".to_string()); // TODO ERROR
    }

    decompress(download, temp_dir, install_dir).unwrap();
    return Ok(());
}
use std::io::{Error, ErrorKind};

fn decompress(d: Download, path: PathBuf, install_path: PathBuf) -> Result<(), io::Error> {
    let file = File::open(&path)
        .or_else(|err| {
            Err(format!(
                "Failed to open file '{}'. Error : {:?}",
                path.to_str().unwrap(),
                err
            ))
        })
        .unwrap();
    let mut archive = Archive::new(GzDecoder::new(file));

    println!("Unpacking files into install location. Please wait");
    let res = archive.unpack(install_path);
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

async fn download_file_progress(
    url: &String,
    total_size: &u64,
    install_dir: &PathBuf,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, "protonup-rs-dev")
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    let total_size = *total_size;
    let total_size_header = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;
    println!("{}", total_size);
    // let total_size = total_size;
    println!("{}", total_size);

    println!("{}", url);
    // Indicatif setup
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").unwrap()
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", &url));

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
        pb.set_position(new);
    }

    pb.finish_with_message(format!(
        "Downloaded {} to {}",
        url,
        install_dir.to_str().unwrap()
    ));
    return Ok(());
}

async fn download_file_into_memory(url: &String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, "protonup-rs-dev")
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    res.text()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
