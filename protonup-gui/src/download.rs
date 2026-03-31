//! GUI-adapted download module that wraps libprotonup with progress reporting.
//! 
//! This module provides functions that report progress through callback functions,
//! which are then sent through the sipper's progress channel.

use anyhow::{Context, Result};
use libprotonup::{
    apps,
    downloads::{self, Download, Release},
    files, hashing,
    sources::CompatTool,
};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, ReadBuf};

use crate::download_task::SipProgress;

/// Download phase enum - shared with download_task
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DownloadPhase {
    #[default]
    DetectingApps,
    FetchingReleases,
    Downloading,
    Validating,
    Unpacking,
    Complete,
    Error,
}

/// Wraps an AsyncWrite to report progress
pub struct ProgressWriter<W, F> {
    inner: W,
    send_progress: F,
    written: u64,
    total: u64,
    tool: String,
    phase: DownloadPhase,
}

impl<W, F> ProgressWriter<W, F> {
    pub fn new(
        inner: W,
        send_progress: F,
        total: u64,
        tool: String,
        phase: DownloadPhase,
    ) -> Self {
        Self {
            inner,
            send_progress,
            written: 0,
            total,
            tool,
            phase,
        }
    }
}

impl<W: AsyncWrite + Unpin, F: Fn(SipProgress) + Send + Unpin> AsyncWrite for ProgressWriter<W, F> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        let result = Pin::new(&mut this.inner).poll_write(cx, buf);

        if let Poll::Ready(Ok(bytes)) = &result {
            this.written += *bytes as u64;
            let percent = if this.total > 0 {
                100.0 * this.written as f32 / this.total as f32
            } else {
                0.0
            };
            (this.send_progress)(SipProgress::new(
                this.phase.clone(),
                &this.tool,
                &format!("Downloading {}... {:.1}%", this.tool, percent),
                percent,
            ));
        }

        result
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

/// Wraps an AsyncRead to report progress
pub struct ProgressReader<R, F> {
    inner: R,
    send_progress: F,
    read: u64,
    total: u64,
    tool: String,
    phase: DownloadPhase,
}

impl<R, F> ProgressReader<R, F> {
    pub fn new(
        inner: R,
        send_progress: F,
        total: u64,
        tool: String,
        phase: DownloadPhase,
    ) -> Self {
        Self {
            inner,
            send_progress,
            read: 0,
            total,
            tool,
            phase,
        }
    }
}

impl<R: AsyncRead + Unpin, F: Fn(SipProgress) + Send + Unpin> AsyncRead for ProgressReader<R, F> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let result = Pin::new(&mut this.inner).poll_read(cx, buf);
        let after = buf.filled().len();

        if let Poll::Ready(Ok(())) = &result {
            let bytes_read = (after - before) as u64;
            this.read += bytes_read;
            let percent = if this.total > 0 {
                100.0 * this.read as f32 / this.total as f32
            } else {
                0.0
            };
            let status_msg = match this.phase {
                DownloadPhase::Validating => format!("Validating {}... {:.1}%", this.tool, percent),
                DownloadPhase::Unpacking => format!("Installing {}... {:.1}%", this.tool, percent),
                _ => format!("{} {:.1}%", this.tool, percent),
            };
            (this.send_progress)(SipProgress::new(
                this.phase.clone(),
                &this.tool,
                &status_msg,
                percent,
            ));
        }

        result
    }
}

/// Main entry point: runs quick downloads with progress reporting through callback
pub async fn run_with_progress_callback<F>(
    send_progress: F,
    force: bool,
) -> Result<Vec<Release>>
where
    F: Fn(SipProgress) + Send + Sync + Clone + Unpin + 'static,
{
    // Phase 1: Detect apps
    send_progress(SipProgress::new(
        DownloadPhase::DetectingApps,
        "",
        "Detecting installed apps...",
        0.0,
    ));

    let found_apps = apps::list_installed_apps().await;
    if found_apps.is_empty() {
        send_progress(SipProgress::new(
            DownloadPhase::Error,
            "",
            "No apps found. Please install Steam or Lutris.",
            0.0,
        ));
        return Err(anyhow::anyhow!(
            "No apps found. Please install at least one app before using this feature."
        ));
    }

    let apps_info: String = found_apps
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    send_progress(SipProgress::new(
        DownloadPhase::FetchingReleases,
        "",
        &format!("Found: {}", apps_info),
        10.0,
    ));

    // Process each app
    let mut releases: Vec<Release> = vec![];
    let total_apps = found_apps.len();

    for (app_index, app_inst) in found_apps.into_iter().enumerate() {
        let compat_tool = app_inst.as_app().default_compatibility_tool();
        let tool_name = compat_tool.name.clone();

        send_progress(SipProgress::new(
            DownloadPhase::FetchingReleases,
            &tool_name,
            &format!("Fetching releases for {}...", tool_name),
            10.0 + (app_index as f32 / total_apps as f32) * 10.0,
        ));

        // Get the latest release
        let release = match downloads::list_releases(&compat_tool).await {
            Ok(mut release_list) => {
                if release_list.is_empty() {
                    send_progress(SipProgress::new(
                        DownloadPhase::Error,
                        &tool_name,
                        &format!("No releases found for {}", tool_name),
                        0.0,
                    ));
                    continue;
                }
                release_list.remove(0)
            }
            Err(e) => {
                send_progress(SipProgress::new(
                    DownloadPhase::Error,
                    &tool_name,
                    &format!("Failed to fetch releases: {}", e),
                    0.0,
                ));
                return Err(anyhow::anyhow!("Failed to fetch releases: {}", e));
            }
        };

        // Handle tools with multiple architecture variants
        let download = if compat_tool.has_multiple_asset_variations {
            let variants = release.get_all_download_variants(&app_inst, &compat_tool);
            variants.into_iter().next().unwrap_or_else(|| {
                release.get_download_info(&app_inst, &compat_tool)
            })
        } else {
            release.get_download_info(&app_inst, &compat_tool)
        };

        // Check if already installed
        let mut download_path = PathBuf::from(&app_inst.default_install_dir().as_str());
        download_path.push(compat_tool.installation_name(&download.version));
        if files::check_if_exists(&download_path.clone()).await && !force {
            send_progress(SipProgress::new(
                DownloadPhase::Complete,
                &tool_name,
                &format!("{} {} already installed, skipping", tool_name, download.version),
                100.0,
            ));
            continue;
        }

        releases.push(release.clone());

        // Download, validate, and unpack
        let progress_callback = send_progress.clone();
        let tool_name_clone = tool_name.clone();
        
        match download_validate_unpack_with_progress(
            download,
            app_inst,
            compat_tool,
            progress_callback,
        ).await {
            Ok(()) => {
                // Continue with next app
            }
            Err(e) => {
                send_progress(SipProgress::new(
                    DownloadPhase::Error,
                    &tool_name_clone,
                    &format!("Error installing {}: {}", tool_name_clone, e),
                    0.0,
                ));
                // Continue with other apps
            }
        }
    }

    // Mark complete
    send_progress(SipProgress::new(
        DownloadPhase::Complete,
        "",
        &format!("Done! Installed {} tools.", releases.len()),
        100.0,
    ));

    Ok(releases)
}

/// Download, validate, and unpack a single tool with progress reporting
async fn download_validate_unpack_with_progress<F>(
    download: Download,
    for_app: apps::AppInstallations,
    compat_tool: CompatTool,
    send_progress: F,
) -> Result<()>
where
    F: Fn(SipProgress) + Send + Sync + Clone + Unpin + 'static,
{
    let tool_name = compat_tool.name.clone();
    let install_dir = for_app.installation_dir(&compat_tool).unwrap();

    // Phase: Download
    send_progress(SipProgress::new(
        DownloadPhase::Downloading,
        &tool_name,
        &format!("Downloading {}...", download.version),
        0.0,
    ));

    // Download file
    let file = download_file_with_progress(&download, send_progress.clone()).await?;

    // Phase: Validate
    send_progress(SipProgress::new(
        DownloadPhase::Validating,
        &tool_name,
        &format!("Validating {}...", download.version),
        0.0,
    ));

    // Validate hash if available
    if let Some(ref hash_sum_info) = download.hash_sum {
        let hash_content = downloads::download_file_into_memory(&hash_sum_info.sum_content)
            .await
            .with_context(|| format!("Error getting hash for {}", download.version))?;

        let hash_sum = hashing::HashSums {
            sum_content: hash_content,
            sum_type: hash_sum_info.sum_type.clone(),
        };

        validate_file_with_progress(&download.file_name, &file, hash_sum, send_progress.clone()).await?;
    }

    // Phase: Unpack
    send_progress(SipProgress::new(
        DownloadPhase::Unpacking,
        &tool_name,
        &format!("Installing {}...", download.version),
        0.0,
    ));

    // Remove existing installation if present
    let install_name = compat_tool.installation_name(&download.version);
    let install_path = install_dir.join(&install_name);
    if files::check_if_exists(&install_path).await {
        fs::remove_dir_all(&install_path)
            .await
            .with_context(|| format!("Error removing existing install at {}", install_path.display()))?;
    }

    // Unpack
    let file_metadata = fs::metadata(&file).await?;

    let decompressor = files::Decompressor::from_path(&file)
        .await
        .with_context(|| format!("Error checking file type of {}", file.display()))?;

    let reader = ProgressReader::new(
        decompressor,
        send_progress.clone(),
        file_metadata.len(),
        tool_name.clone(),
        DownloadPhase::Unpacking,
    );

    files::unpack_file(&compat_tool, &download, reader, &install_dir)
        .await
        .with_context(|| format!("Error unpacking {}", file.display()))?;

    // Mark this tool as complete
    send_progress(SipProgress::new(
        DownloadPhase::Complete,
        &tool_name,
        &format!("✓ {} installed successfully", tool_name),
        100.0,
    ));

    Ok(())
}

/// Download file with progress reporting
async fn download_file_with_progress<F>(
    download: &Download,
    send_progress: F,
) -> Result<PathBuf>
where
    F: Fn(SipProgress) + Send + Sync + Clone + Unpin + 'static,
{
    let output_dir = download.download_dir()?;

    if files::check_if_exists(&output_dir).await {
        fs::remove_dir_all(&output_dir).await?;
    }

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&output_dir)
        .await
        .with_context(|| {
            format!(
                "[Download] Failed creating destination file: {}",
                output_dir.display()
            )
        })?;

    let writer = ProgressWriter::new(
        file,
        send_progress,
        download.size,
        download.version.clone(),
        DownloadPhase::Downloading,
    );

    downloads::download_to_async_write(&download.download_url, &mut writer.into_async_write())
        .await?;

    Ok(output_dir)
}

/// Validate file hash with progress reporting
async fn validate_file_with_progress<F>(
    file_name: &str,
    path: &Path,
    hash: hashing::HashSums,
    send_progress: F,
) -> Result<()>
where
    F: Fn(SipProgress) + Send + Sync + Clone + Unpin + 'static,
{
    let file = File::open(path)
        .await
        .context("[Hash Check] Failed opening download file")?;

    let file_size = fs::metadata(path).await?.len();
    let reader = ProgressReader::new(
        BufReader::new(file),
        send_progress,
        file_size,
        file_name.to_string(),
        DownloadPhase::Validating,
    );

    if !hashing::hash_check_file(file_name, &mut reader.into_async_read(), hash).await? {
        anyhow::bail!("{} failed validation", path.display());
    }

    Ok(())
}

// Helper to convert ProgressWriter back to AsyncWrite for download_to_async_write
impl<W: AsyncWrite + Unpin, F: Fn(SipProgress) + Send + Unpin> ProgressWriter<W, F> {
    fn into_async_write(self) -> Self {
        self
    }
}

impl<R: AsyncRead + Unpin, F: Fn(SipProgress) + Send + Unpin> ProgressReader<R, F> {
    fn into_async_read(self) -> Self {
        self
    }
}
