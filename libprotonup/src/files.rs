use std::fmt;
use std::future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::Poll;

use anyhow::{Context, Error, Result, anyhow};
use async_compression::tokio::bufread::{GzipDecoder, XzDecoder, ZstdDecoder};
use futures_util::StreamExt;
use pin_project::pin_project;
use tokio::fs::File;
use tokio::io::{AsyncBufRead, AsyncRead, BufReader, ReadBuf};
use tokio::{fs, io, pin};
use tokio_stream::wrappers::ReadDirStream;
use tokio_tar::ArchiveBuilder;

use crate::downloads::Download;
use crate::sources::CompatTool;
use crate::utils;

#[pin_project(project = DecompressorProject)]
pub enum Decompressor<R: AsyncBufRead + Unpin> {
    Gzip(#[pin] GzipDecoder<R>),
    Xz(#[pin] XzDecoder<R>),
    Zstd(#[pin] ZstdDecoder<R>),
}

pub(crate) fn check_supported_extension(file_name: String) -> Result<String> {
    if file_name.ends_with("tar.gz") || file_name.ends_with("tgz") {
        Ok("tar.gz".to_owned())
    } else if file_name.ends_with("tar.zst") || file_name.ends_with("tar.zstd") {
        Ok("tar.zst".to_owned())
    } else if file_name.ends_with("tar.xz") || file_name.ends_with("txz") {
        Ok("tar.xz".to_owned())
    } else {
        Err(anyhow!(
            "Downloaded file wasn't of the expected type. (tar.(gz/xz/zst))"
        ))
    }
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

        // TODO: implement this using the same validation function
        if path_str.ends_with("tar.gz") {
            Ok(Decompressor::Gzip(GzipDecoder::new(BufReader::new(file))))
        } else if path_str.ends_with("tar.xz") {
            Ok(Decompressor::Xz(XzDecoder::new(BufReader::new(file))))
        } else if path_str.ends_with("tar.zst") {
            Ok(Decompressor::Zstd(ZstdDecoder::new(BufReader::new(file))))
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
            DecompressorProject::Zstd(reader) => {
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
    compat_tool: &CompatTool,
    download: &Download,
    reader: R,
    install_path: &Path,
) -> Result<()> {
    let install_dir = utils::expand_tilde(install_path).unwrap();

    fs::create_dir_all(&install_dir).await.unwrap();

    decompress_with_new_top_level(
        reader,
        install_dir.as_path(),
        compat_tool.installation_name(&download.version).as_str(),
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
    let mut archive = ArchiveBuilder::new(reader)
        .set_unpack_xattrs(false)
        .set_preserve_permissions(true)
        .set_preserve_mtime(true)
        .set_overwrite(true)
        .set_ignore_zeros(false)
        .build();

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
pub async fn check_if_exists(path: &PathBuf) -> bool {
    let f_path = utils::expand_tilde(path).unwrap();
    let p = f_path.as_path();
    fs::metadata(p).await.map(|m| m.is_dir()).unwrap_or(false)
}

/// list_folders_in_path returns a vector of strings of the folders in a path
pub async fn list_folders_in_path(path: &PathBuf) -> Result<Vec<String>, anyhow::Error> {
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
pub async fn remove_dir_all(path: &PathBuf) -> Result<()> {
    let f_path = utils::expand_tilde(path).unwrap();
    let p = f_path.as_path();
    tokio::fs::remove_dir_all(p)
        .await
        .with_context(|| format!("[Remove] Failed to remove directory : {}", p.display()))?;
    Ok(())
}

/// Folder structure is a helper to Display a combo of Path and subpath
pub struct Folder(pub (PathBuf, String));

impl fmt::Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Access the tuple's second element (String) using self.0.1
        write!(f, "{}", self.0.1)
    }
}

/// Folders is just an alias of `Vec<Folder>` to implement Display
pub struct Folders(pub Vec<Folder>);

impl fmt::Display for Folders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, folder) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{folder}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{apps::AppInstallations, sources};

    use super::*;
    use anyhow::Result;
    use std::fs;
    use tar;
    use tempfile::tempdir;

    #[test]
    fn test_check_supported_extension() {
        struct TestCase {
            name: &'static str,
            file_name: String,
            expected: Result<String>,
        }

        let test_cases = vec![
            TestCase {
                name: "tar.gz",
                file_name: "my_archive.tar.gz".to_string(),
                expected: Ok("tar.gz".to_string()),
            },
            TestCase {
                name: "tgz",
                file_name: "another_archive.tgz".to_string(),
                expected: Ok("tar.gz".to_string()),
            },
            TestCase {
                name: "tar.zst",
                file_name: "data.tar.zst".to_string(),
                expected: Ok("tar.zst".to_string()),
            },
            TestCase {
                name: "tar.zstd",
                file_name: "backup.tar.zstd".to_string(),
                expected: Ok("tar.zst".to_string()),
            },
            TestCase {
                name: "tar.xz",
                file_name: "image.tar.xz".to_string(),
                expected: Ok("tar.xz".to_string()),
            },
            TestCase {
                name: "txz",
                file_name: "report.txz".to_string(),
                expected: Ok("tar.xz".to_string()),
            },
            TestCase {
                name: "unsupported extension - zip",
                file_name: "document.zip".to_string(),
                expected: Err(anyhow::anyhow!(
                    "Downloaded file wasn't of the expected type. (tar.(gz/xz/zst))",
                )),
            },
            TestCase {
                name: "unsupported extension - tar",
                file_name: "plain.tar".to_string(),
                expected: Err(anyhow::anyhow!(
                    "Downloaded file wasn't of the expected type. (tar.(gz/xz/zst))",
                )),
            },
            TestCase {
                name: "unsupported extension - no extension",
                file_name: "config".to_string(),
                expected: Err(anyhow::anyhow!(
                    "Downloaded file wasn't of the expected type. (tar.(gz/xz/zst))",
                )),
            },
            TestCase {
                name: "filename with supported extension in the middle",
                file_name: "prefix.tar.gz.suffix".to_string(),
                expected: Err(anyhow::anyhow!(
                    "Downloaded file wasn't of the expected type. (tar.(gz/xz/zst))",
                )),
            },
            TestCase {
                name: "empty filename",
                file_name: "".to_string(),
                expected: Err(anyhow::anyhow!(
                    "Downloaded file wasn't of the expected type. (tar.(gz/xz/zst))",
                )),
            },
        ];

        for test_case in test_cases {
            let result = check_supported_extension(test_case.file_name);
            match (result, test_case.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected, "Test '{}' failed", test_case.name)
                }
                (Err(actual), Err(expected)) => {
                    assert_eq!(
                        actual.to_string(),
                        expected.to_string(),
                        "Test '{}' failed",
                        test_case.name
                    );
                }
                (Ok(_), Err(_)) => panic!(
                    "Test '{}' failed: Expected error, got success",
                    test_case.name
                ),
                (Err(_), Ok(_)) => panic!(
                    "Test '{}' failed: Expected success, got error",
                    test_case.name
                ),
            }
        }
    }

    #[tokio::test]
    async fn test_unpack_with_new_top_level() {
        let empty = "".to_owned();

        let s = CompatTool::new_custom(
            empty.clone(),
            sources::Forge::GitHub,
            empty.clone(),
            empty.clone(),
            sources::ToolType::WineBased,
            None,
            None,
            None,
        );

        let d = Download {
            file_name: "test".to_owned(),
            for_app: AppInstallations::Steam,
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

        unpack_file(&s, &d, file, output_dir.as_path())
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
