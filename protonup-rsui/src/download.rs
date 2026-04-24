//! GUI-adapted download module that wraps libprotonup with progress reporting.
//!
//! This module provides functions that report progress through callback functions,
//! which are then sent through the sipper's progress channel.

use anyhow::{Context, Result};
use libprotonup::{
    apps::{self, AppInstallations},
    architecture_variants,
    downloads::{self, Download, Release},
    files, hashing,
    sources::CompatTool,
};
use std::collections::HashSet;
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
    pub fn new(inner: W, send_progress: F, total: u64, tool: String, phase: DownloadPhase) -> Self {
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
    pub fn new(inner: R, send_progress: F, total: u64, tool: String, phase: DownloadPhase) -> Self {
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
pub async fn run_with_progress_callback<F>(send_progress: F, force: bool) -> Result<Vec<Release>>
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

    // Collect all downloads first, then run them in parallel
    let mut downloads_to_run: Vec<(Download, apps::AppInstallations, CompatTool, Release)> = vec![];
    let mut releases: Vec<Release> = vec![];

    // Phase 1: Fetch releases and prepare downloads
    for app_inst in &found_apps {
        let compat_tool = app_inst.as_app().default_compatibility_tool();
        // Use tool name + app for unique matching with GUI tool entries
        let tool_name = compat_tool.name.clone();
        let tool_id = format!("{} ({})", tool_name, app_inst.as_app());

        send_progress(SipProgress::new(
            DownloadPhase::FetchingReleases,
            &tool_id,
            &format!("Fetching releases for {}...", tool_name),
            10.0,
        ));

        // Get the latest release
        let release = match downloads::list_releases(&compat_tool).await {
            Ok(mut release_list) => {
                if release_list.is_empty() {
                    send_progress(SipProgress::new(
                        DownloadPhase::Error,
                        &tool_id,
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
                    &tool_id,
                    &format!("Failed to fetch releases: {}", e),
                    0.0,
                ));
                return Err(anyhow::anyhow!("Failed to fetch releases: {}", e));
            }
        };

        // Handle tools with multiple architecture variants
        let download = if compat_tool.has_multiple_asset_variations {
            let variants = release.get_all_download_variants(app_inst, &compat_tool);
            variants
                .into_iter()
                .next()
                .unwrap_or_else(|| release.get_download_info(app_inst, &compat_tool))
        } else {
            release.get_download_info(app_inst, &compat_tool)
        };

        // Check if already installed
        let mut download_path = PathBuf::from(&app_inst.default_install_dir().as_str());
        download_path.push(compat_tool.installation_name(&download.version));
        if files::check_if_exists(&download_path.clone()).await && !force {
            send_progress(SipProgress::new(
                DownloadPhase::Complete,
                &tool_id,
                &format!(
                    "{} {} already installed, skipping",
                    tool_name, download.version
                ),
                100.0,
            ));
            continue;
        }

        releases.push(release.clone());
        downloads_to_run.push((download, app_inst.clone(), compat_tool, release));
    }

    // Phase 2: Run all downloads in parallel using tokio::spawn
    send_progress(SipProgress::new(
        DownloadPhase::Downloading,
        "",
        &format!(
            "Downloading {} tools in parallel...",
            downloads_to_run.len()
        ),
        20.0,
    ));

    let mut handles = Vec::new();

    for (download, app_inst, compat_tool, _release) in downloads_to_run {
        let progress_callback = send_progress.clone();
        // Use tool name + app to match GUI ToolDownload entries
        let display_name: String = format!("{} ({})", compat_tool.name, app_inst.as_app());

        handles.push(tokio::spawn(async move {
            let result = download_validate_unpack_with_progress(
                download,
                app_inst,
                compat_tool,
                progress_callback,
                display_name.clone(),
            )
            .await;
            (display_name, result)
        }));
    }

    // Process results as they complete
    let mut success_count = 0;
    for handle in handles {
        let result: Result<(String, Result<(), anyhow::Error>), _> = handle.await;
        match result {
            Ok((_tool_name, Ok(()))) => {
                success_count += 1;
            }
            Ok((tool_name, Err(e))) => {
                send_progress(SipProgress::new(
                    DownloadPhase::Error,
                    &tool_name,
                    &format!("Error installing {}: {}", tool_name, e),
                    0.0,
                ));
            }
            Err(join_err) => {
                send_progress(SipProgress::new(
                    DownloadPhase::Error,
                    "unknown",
                    &format!("Task failed: {}", join_err),
                    0.0,
                ));
            }
        }
    }

    // Mark complete
    send_progress(SipProgress::new(
        DownloadPhase::Complete,
        "",
        &format!(
            "Done! Installed {}/{} tools.",
            success_count,
            releases.len()
        ),
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
    display_name: String, // e.g., "GEProton GE-Proton9-27"
) -> Result<()>
where
    F: Fn(SipProgress) + Send + Sync + Clone + Unpin + 'static,
{
    let install_dir = for_app.installation_dir(&compat_tool).unwrap();

    // Phase: Download
    send_progress(SipProgress::new(
        DownloadPhase::Downloading,
        &display_name,
        &format!("Downloading {}...", download.version),
        0.0,
    ));

    // Download file
    let file =
        download_file_with_progress(&download, send_progress.clone(), display_name.clone()).await?;

    // Phase: Validate
    send_progress(SipProgress::new(
        DownloadPhase::Validating,
        &display_name,
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

        validate_file_with_progress(&download.file_name, &file, hash_sum, send_progress.clone())
            .await?;
    }

    // Phase: Unpack
    send_progress(SipProgress::new(
        DownloadPhase::Unpacking,
        &display_name,
        &format!("Installing {}...", download.version),
        0.0,
    ));

    // Remove existing installation if present
    let install_name = compat_tool.installation_name(&download.version);
    let install_path = install_dir.join(&install_name);
    if files::check_if_exists(&install_path).await {
        fs::remove_dir_all(&install_path).await.with_context(|| {
            format!(
                "Error removing existing install at {}",
                install_path.display()
            )
        })?;
    }

    // Unpack
    let file_metadata = fs::metadata(&file).await?;

    // Open the compressed file
    let compressed_file = File::open(&file)
        .await
        .with_context(|| format!("Error opening compressed file {}", file.display()))?;

    // Wrap the file with ProgressReader to track compressed bytes read
    let progress_reader = ProgressReader::new(
        compressed_file,
        send_progress.clone(),
        file_metadata.len(),
        display_name.clone(),
        DownloadPhase::Unpacking,
    );

    // Wrap with BufReader to provide AsyncBufRead for the decompressor
    let buf_reader = BufReader::new(progress_reader);

    // Create Decompressor from the BufReader<ProgressReader<File>>
    let path_str = file.to_string_lossy();
    let decompressor = files::Decompressor::from_reader(buf_reader, &path_str)
        .with_context(|| format!("Error checking file type of {}", file.display()))?;

    files::unpack_file(&compat_tool, &download, decompressor, &install_dir)
        .await
        .with_context(|| format!("Error unpacking {}", file.display()))?;

    // Mark this tool as complete
    send_progress(SipProgress::new(
        DownloadPhase::Complete,
        &display_name,
        &format!("✓ {} installed successfully", display_name),
        100.0,
    ));

    Ok(())
}

/// Download file with progress reporting
async fn download_file_with_progress<F>(
    download: &Download,
    send_progress: F,
    display_name: String,
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
        display_name,
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

/// Fetch releases for a compatibility tool
pub async fn fetch_releases(tool: CompatTool) -> Vec<Release> {
    match downloads::list_releases(&tool).await {
        Ok(releases) => releases,
        Err(e) => {
            eprintln!("Failed to fetch releases: {}", e);
            vec![]
        }
    }
}

/// Download specific tools and versions for an app
pub async fn download_selected_tools<F>(
    app_installation: AppInstallations,
    tools_and_versions: Vec<(CompatTool, Vec<Release>)>,
    send_progress: F,
    force_reinstall_names: HashSet<String>,
    arch_variant: Option<u8>,
) -> Result<Vec<Release>>
where
    F: Fn(SipProgress) + Send + Sync + Clone + Unpin + 'static,
{
    let mut releases: Vec<Release> = vec![];
    let mut downloads_to_run: Vec<(Download, AppInstallations, CompatTool, Release)> = vec![];

    // Phase 1: Prepare all downloads
    send_progress(SipProgress::global(
        DownloadPhase::FetchingReleases,
        "Preparing downloads...",
        10.0,
    ));

    for (compat_tool, versions) in &tools_and_versions {
        for release in versions {
            // Handle tools with multiple architecture variants
            let download = if compat_tool.has_multiple_asset_variations {
                let variants = release.get_all_download_variants(&app_installation, compat_tool);

                // Select variant based on arch_variant parameter
                if let Some(variant_code) = arch_variant {
                    let variant_name = architecture_variants::get_variant_name(variant_code);
                    variants
                        .into_iter()
                        .find(|d| d.file_name.contains(variant_name))
                        .unwrap_or_else(|| {
                            release.get_download_info(&app_installation, compat_tool)
                        })
                } else {
                    // Default to v2 or first available
                    architecture_variants::select_default_variant(&variants).unwrap_or_else(|| {
                        release.get_download_info(&app_installation, compat_tool)
                    })
                }
            } else {
                release.get_download_info(&app_installation, compat_tool)
            };

            // Check if already installed (only if not in force reinstall set)
            let display_name = format!("{} {}", compat_tool.name, release.tag_name);
            let mut download_path = PathBuf::from(&app_installation.default_install_dir().as_str());
            download_path.push(compat_tool.installation_name(&download.version));

            if !force_reinstall_names.contains(&display_name)
                && files::check_if_exists(&download_path.clone()).await
            {
                send_progress(SipProgress::new(
                    DownloadPhase::Complete,
                    &display_name,
                    &format!("{} already installed, skipping", display_name),
                    100.0,
                ));
                continue;
            }

            releases.push(release.clone());
            downloads_to_run.push((
                download,
                app_installation.clone(),
                compat_tool.clone(),
                release.clone(),
            ));
        }
    }

    if downloads_to_run.is_empty() {
        send_progress(SipProgress::global(
            DownloadPhase::Complete,
            "All tools already installed or nothing to download.",
            100.0,
        ));
        return Ok(releases);
    }

    // Phase 2: Run all downloads in parallel using tokio::spawn
    send_progress(SipProgress::global(
        DownloadPhase::Downloading,
        &format!(
            "Downloading {} tools in parallel...",
            downloads_to_run.len()
        ),
        20.0,
    ));

    let mut handles = Vec::new();

    for (download, app_inst, compat_tool, release) in downloads_to_run {
        let progress_callback = send_progress.clone();
        // Use combined tool name + version to match GUI ToolDownload entries
        let display_name: String = format!("{} {}", compat_tool.name, release.tag_name);

        handles.push(tokio::spawn(async move {
            let result = download_validate_unpack_with_progress(
                download,
                app_inst,
                compat_tool,
                progress_callback,
                display_name.clone(),
            )
            .await;
            (display_name, result)
        }));
    }

    // Process results as they complete
    let mut success_count = 0;
    for handle in handles {
        let result: Result<(String, Result<(), anyhow::Error>), _> = handle.await;
        match result {
            Ok((_tool_name, Ok(()))) => {
                success_count += 1;
            }
            Ok((tool_name, Err(e))) => {
                send_progress(SipProgress::new(
                    DownloadPhase::Error,
                    &tool_name,
                    &format!("Error installing {}: {}", tool_name, e),
                    0.0,
                ));
            }
            Err(join_err) => {
                send_progress(SipProgress::new(
                    DownloadPhase::Error,
                    "unknown",
                    &format!("Task failed: {}", join_err),
                    0.0,
                ));
            }
        }
    }

    // Mark complete
    send_progress(SipProgress::global(
        DownloadPhase::Complete,
        &format!(
            "Done! Installed {}/{} tools.",
            success_count,
            releases.len()
        ),
        100.0,
    ));

    Ok(releases)
}
