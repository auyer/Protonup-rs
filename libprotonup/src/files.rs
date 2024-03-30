use std::future;
use std::path::Path;
use std::pin::Pin;
use std::task::Poll;

use anyhow::{Context, Error, Result};
use async_compression::tokio::bufread::{GzipDecoder, XzDecoder};
use futures_util::{StreamExt, TryStreamExt};
use pin_project::pin_project;
use reqwest::header::USER_AGENT;
use sha2::{Digest, Sha512};
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, BufReader, ReadBuf};
use tokio::{fs, io, pin};
use tokio_stream::wrappers::ReadDirStream;
use tokio_tar::Archive;
use tokio_util::io::StreamReader;

use crate::utils;

use super::constants;

#[pin_project(project = DecompressorProject)]
pub enum Decompressor<R: AsyncBufRead + Unpin> {
    Gzip(#[pin] GzipDecoder<R>),
    Xz(#[pin] XzDecoder<R>),
}

impl Decompressor<BufReader<File>> {
    pub async fn from_path(path: &Path) -> Result<Self> {
        let path_str = path.as_os_str().to_string_lossy();

        let file = File::open(path).await.with_context(|| {
            format!(
                "[Decompressing] Failed to unpack into destination : {}",
                path.display()
            )
        })?;

        if path_str.ends_with("tar.gz") {
            Ok(Decompressor::Gzip(GzipDecoder::new(BufReader::new(file))))
        } else if path_str.ends_with("tar.xz") {
            Ok(Decompressor::Xz(XzDecoder::new(BufReader::new(file))))
        } else {
            Err(Error::msg(format!(
                "no decompress\nPath: {}",
                path.display()
            )))
        }
    }
}

impl<R: AsyncBufRead + Unpin> AsyncRead for Decompressor<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.project() {
            DecompressorProject::Gzip(reader) => {
                pin!(reader);
                reader.poll_read(cx, buf)
            }
            DecompressorProject::Xz(reader) => {
                pin!(reader);
                reader.poll_read(cx, buf)
            }
        }
    }
}

/// decompress will detect the extension and decompress the file with the appropriate function
pub async fn decompress<R: AsyncRead + Unpin>(reader: R, destination_path: &Path) -> Result<()> {
    let mut archive = Archive::new(reader);

    archive
        .unpack(destination_path)
        .await
        .with_context(|| decompress_context(destination_path))
}

/// check_if_exists checks if a folder exists in a path
pub async fn check_if_exists(path: &str, tag: &str) -> bool {
    let f_path = utils::expand_tilde(format!("{path}{tag}/")).unwrap();
    let p = f_path.as_path();
    fs::metadata(p).await.map(|m| m.is_dir()).unwrap_or(false)
}

/// list_folders_in_path returns a vector of strings of the folders in a path
pub async fn list_folders_in_path(path: &str) -> Result<Vec<String>, anyhow::Error> {
    let f_path = utils::expand_tilde(path).unwrap();
    let paths_real: Vec<String> = ReadDirStream::new(tokio::fs::read_dir(f_path).await?)
        .filter_map(|e| future::ready(e.ok()))
        .filter(|e| future::ready(e.path().is_dir()))
        .map(|e| e.path().file_name().unwrap().to_str().unwrap().to_string())
        .collect()
        .await;
    Ok(paths_real)
}

/// Removes a directory and all its contents
pub async fn remove_dir_all(path: &str) -> Result<()> {
    let f_path = utils::expand_tilde(path).unwrap();
    let p = f_path.as_path();
    tokio::fs::remove_dir_all(p)
        .await
        .with_context(|| format!("[Remove] Failed to remove directory : {}", p.display()))?;
    Ok(())
}

pub async fn download_to_async_write<W: AsyncWrite + Unpin>(
    url: &str,
    write: &mut W,
) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, format!("protonup-rs {}", constants::VERSION))
        .send()
        .await
        .with_context(|| format!("[Download] Failed to call remote server on URL : {}", &url))?;

    io::copy(
        &mut StreamReader::new(
            res.bytes_stream()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
        ),
        write,
    )
    .await?;
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
pub async fn hash_check_file<R: AsyncRead + Unpin + ?Sized>(
    reader: &mut R,
    git_hash: &str,
) -> Result<bool> {
    let mut hasher = Sha512::new();

    read_all_into_digest(reader, &mut hasher)
        .await
        .context("[Hash Check] Failed reading download file for checking")?;

    let hash = hasher.finalize();

    let (git_hash, _) = git_hash.rsplit_once(' ').unwrap_or((git_hash, ""));

    if hex::encode(hash) != git_hash.trim() {
        return Ok(false);
    }
    Ok(true)
}

async fn read_all_into_digest<R: AsyncRead + Unpin + ?Sized, D: Digest>(
    read: &mut R,
    digest: &mut D,
) -> Result<()> {
    const BUFFER_LEN: usize = 8 * 1024; // 8KB
    let mut buffer = [0u8; BUFFER_LEN];

    loop {
        let count = read.read(&mut buffer).await?;
        digest.update(&buffer[..count]);
        if count != BUFFER_LEN {
            break;
        }
    }

    Ok(())
}

fn decompress_context(destination_path: &Path) -> String {
    format!(
        "[Decompressing] Failed to unpack into destination : {}",
        destination_path.display()
    )
}

#[cfg(test)]
mod test {
    use sha2::{Digest, Sha512};

    #[tokio::test]
    async fn hash_check_file() {
        let test_data = b"This Is A Test";
        let hash = hex::encode(Sha512::new_with_prefix(&test_data).finalize());

        assert!(
            super::hash_check_file(&mut &test_data[..], &hash)
                .await
                .unwrap(),
            "Hash didn't match"
        );
    }
}
