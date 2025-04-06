use std::future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::Poll;

use anyhow::{Context, Error, Result};
use async_compression::tokio::bufread::{GzipDecoder, XzDecoder};
use futures_util::{StreamExt, TryStreamExt};
use pin_project::pin_project;
use reqwest::header::USER_AGENT;
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, BufReader, ReadBuf};
use tokio::{fs, io, pin};
use tokio_stream::wrappers::ReadDirStream;
use tokio_tar::Archive;
use tokio_util::io::StreamReader;

use crate::github::Download;
use crate::sources::Source;
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

/// Prepares downloaded file to be decompressed
///
/// Parses the passed in data and ensures the destination directory is created
pub async fn unpack_file<R: AsyncRead + Unpin>(
    source: Source,
    download: Download,
    reader: R,
    install_path: &str,
) -> Result<()> {
    let install_dir = utils::expand_tilde(install_path).unwrap();

    fs::create_dir_all(&install_dir).await.unwrap();

    decompress_with_new_top_level(
        reader,
        install_dir.as_path(),
        download.output_dir(&source).as_str(),
    )
    .await
    .unwrap();

    Ok(())
}

/// decompress_with_new_top_level unpacks the tarrball,
/// replacing the top level folder with the provided value
async fn decompress_with_new_top_level<R: AsyncRead + Unpin>(
    reader: R,
    destination_path: &Path,
    new_top_level: &str,
) -> Result<()> {
    let mut archive = Archive::new(reader);

    // Get the entries from the archive
    let mut entries = archive.entries()?;

    while let Some(entry) = entries.next().await {
        let mut entry = entry?;

        // Get the original path in the tar
        let path = entry.path()?;

        // Create the new path by replacing the top level
        let new_path = if path.parent().is_some() {
            let components: Vec<_> = path.components().collect();
            // skip len 1, it corresponds to the top level itself
            if components.len() > 1 {
                let mut new_path = PathBuf::from(destination_path).join(new_top_level);
                for component in components.iter().skip(1) {
                    new_path.push(component);
                }
                new_path
            } else {
                PathBuf::from(destination_path)
                    .join(new_top_level)
                    .join(path.file_name().unwrap())
            }
        } else {
            PathBuf::from(destination_path)
                .join(new_top_level)
                .join(path)
        };

        // Create parent directories if needed
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Extract the file
        entry.unpack(&new_path).await?;
    }

    Ok(())
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

#[cfg(test)]
mod test {
    use crate::sources;

    use super::*;
    use std::fs;
    use tar;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_unpack_with_new_top_level() {
        let empty = "".to_owned();

        let s = Source::new_custom(
            empty.clone(),
            sources::Forge::GitHub,
            empty.clone(),
            empty.clone(),
            None,
            None,
        );

        let d = Download {
            version: "new_top_123".to_owned(),
            hash_sum: None,
            download_url: "test.tar".to_owned(),
            size: 1,
        };

        // Create a temporary directory for the test
        let temp_dir = tempdir().unwrap();
        let tar_path = temp_dir.path().join("test.tar");
        let output_dir = temp_dir.path().join("./output");
        let new_top_level = d.version.clone();

        // Create sample directory structure to tar
        let original_top = temp_dir.path().join("original_top");
        fs::create_dir_all(&original_top).unwrap();
        fs::write(original_top.join("file1.txt"), "test content").unwrap();
        fs::create_dir_all(original_top.join("subdir")).unwrap();
        fs::write(original_top.join("subdir/file2.txt"), "more content").unwrap();

        // Create a tar file
        let tar_file = fs::File::create(&tar_path).unwrap();
        let mut builder = tar::Builder::new(tar_file);
        builder
            .append_dir_all("original_top", &original_top)
            .unwrap();
        builder.finish().unwrap();

        let file = File::open(tar_path).await.unwrap();

        unpack_file(s, d, file, output_dir.to_str().unwrap())
            .await
            .expect("Unpacking failed");

        // Verify the new directory structure
        let new_root = output_dir.join(new_top_level);
        assert!(new_root.exists(), "New top level directory not created");

        let file1 = new_root.join("file1.txt");
        assert!(file1.is_file(), "File not found in new structure");
        assert_eq!(fs::read_to_string(file1).unwrap(), "test content");

        let file2 = new_root.join("subdir/file2.txt");
        assert!(file2.is_file(), "Nested file not found");
        assert_eq!(fs::read_to_string(file2).unwrap(), "more content");
    }
}
