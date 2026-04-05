# libprotonup

Core library for automating the installation and update of Linux Gaming Compatibility tools like ProtonGE, Luxtorpeda, Boxtron, and others.

## Overview

`libprotonup` provides the core functionality for discovering, downloading, validating, and installing compatibility tools for Steam and Lutris. It handles:

- **App Detection**: Finding Steam and Lutris installations (native and Flatpak)
- **Release Discovery**: Fetching available versions from GitHub releases
- **Download Management**: Streaming downloads with progress tracking
- **Hash Verification**: SHA256/SHA512 integrity checking
- **Archive Extraction**: Unpacking tar.gz, tar.xz, and tar.zst files
- **Installation**: Proper directory structure for each tool

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Application Layer                        │
│                    (protonup-rs CLI / GUI)                       │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         libprotonup                              │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌───────────────┐  │
│  │  apps    │  │ sources  │  │ downloads │  │    files      │  │
│  │          │  │          │  │           │  │               │  │
│  │ - detect │  │ - config │  │ - fetch   │  │ - unpack      │  │
│  │ - paths  │  │ - tools  │  │ - stream  │  │ - validate    │  │
│  └──────────┘  └──────────┘  └───────────┘  └───────────────┘  │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐                     │
│  │ hashing  │  │  utils   │  │ constants │                     │
│  │          │  │          │  │           │                     │
│  │ - sha256 │  │ - temp   │  │ - paths   │                     │
│  │ - sha512 │  │ - tilde  │  │ - config  │                     │
│  └──────────┘  └──────────┘  └───────────┘                     │
└─────────────────────────────────────────────────────────────────┘
```

## Modules

### `apps` - Application Detection

Handles detection of Steam and Lutris installations.

**Key Types:**
- `App` - Enum representing supported applications (Steam, Lutris, Custom)
- `AppInstallations` - Specific installation variants (Native, Flatpak, Custom)

**Key Functions:**
```rust
// Detect all installed apps
pub async fn list_installed_apps() -> Vec<AppInstallations>

// Detect installation methods for a specific app
pub async fn detect_installation_method(&self) -> Vec<AppInstallations>

// Get installation directory for a compatibility tool
pub fn installation_dir(&self, compat_tool: &CompatTool) -> Option<PathBuf>

// List installed versions in the installation directory
pub async fn list_installed_versions(&self) -> Result<Vec<Folder>>
```

**Installation Paths:**
| App | Variant | Base Directory |
|-----|---------|----------------|
| Steam | Native | `~/.steam/steam/` |
| Steam | Flatpak | `~/.var/app/com.valvesoftware.Steam/data/Steam/` |
| Lutris | Native | `~/.local/share/lutris/` |
| Lutris | Flatpak | `~/.var/app/net.lutris.Lutris/data/lutris/` |

### `sources` - Compatibility Tools Configuration

Defines available compatibility tools and their sources.

**Key Types:**
- `CompatTool` - Configuration for a compatibility tool
- `CompatTools` - Lazy-static list of all available tools (from `sources.ron`)
- `Forge` - Source platform (GitHub, Custom URL)
- `ToolType` - Installation type (WineBased, Runtime)

**Key Functions:**
```rust
// Get tools compatible with a specific app
pub fn sources_for_app(app: &App) -> Vec<CompatTool>

// Get installation name for a version
pub fn installation_name(&self, version: &str) -> String

// Filter release assets by regex pattern
pub fn filter_asset(&self, path: &str) -> bool
```

**Configuration (sources.ron):**
```ron
[
    CompatTool(
        name: "GEProton",
        forge: GitHub,
        repository_account: "GloriousEggroll",
        repository_name: "proton-ge-custom",
        compatible_applications: [Steam],
        tool_type: Runtime,
        release_asset_filter: Some(r"^(GE-Proton|Proton-)[0-9]+.*\.(tar\.gz|tar\.zst)$"),
        file_name_replacement: None,
        file_name_template: None,
        has_multiple_asset_variations: false,
    ),
    // ... more tools
]
```

### `downloads` - Download Management

Handles fetching release information and downloading files.

**Key Types:**
- `Release` - GitHub release information
- `Asset` - Individual release asset (download file)
- `Download` - Prepared download with URL, size, and hash info

**Key Functions:**
```rust
// List all releases for a compatibility tool
pub async fn list_releases(compat_tool: &CompatTool) -> Result<ReleaseList>

// Get download info for a specific release
pub fn get_download_info(
    &self,
    for_app: &AppInstallations,
    compat_tool: &CompatTool,
) -> Download

// Get all architecture variants for a release
pub fn get_all_download_variants(
    &self,
    for_app: &AppInstallations,
    compat_tool: &CompatTool,
) -> Vec<Download>

// Download to async writer (supports progress wrapping)
pub async fn download_to_async_write<W: AsyncWrite + Unpin>(
    url: &str,
    write: &mut W,
) -> Result<()>

// Download hash file content
pub async fn download_file_into_memory(url: &String) -> Result<String>
```

**Download Struct:**
```rust
pub struct Download {
    pub file_name: String,           // Archive filename
    pub for_app: AppInstallations,   // Target application
    pub version: String,             // Version tag
    pub hash_sum: Option<HashSums>,  // Optional hash for verification
    pub download_url: String,        // Direct download URL
    pub size: u64,                   // File size in bytes
}
```

### `files` - File Operations

Handles archive extraction and file system operations.

**Key Types:**
- `Decompressor<R>` - Enum for different compression formats (Gzip, Xz, Zstd)
- `Folder` - Helper type for directory listings

**Key Functions:**
```rust
// Create decompressor from file path
pub async fn from_path(path: &Path) -> Result<Decompressor<BufReader<File>>>

// Unpack archive to installation directory
pub async fn unpack_file<R: AsyncRead + Unpin>(
    compat_tool: &CompatTool,
    download: &Download,
    reader: R,
    install_path: &Path,
) -> Result<()>

// Check if directory exists
pub async fn check_if_exists(path: &PathBuf) -> bool

// List folders in directory
pub async fn list_folders_in_path(path: &PathBuf) -> Result<Vec<String>>

// Validate file extension
pub fn check_supported_extension(file_name: &str) -> Result<String>
```

**Supported Formats:**
- `.tar.gz` / `.tgz`
- `.tar.xz` / `.txz`
- `.tar.zst` / `.tar.zstd`

### `hashing` - Hash Verification

Handles integrity verification of downloaded files.

**Key Types:**
- `HashSums` - Hash content and type
- `HashSumType` - Enum (Sha256, Sha512)

**Key Functions:**
```rust
// Verify file hash against expected value
pub async fn hash_check_file<R: AsyncRead + Unpin + ?Sized>(
    file_name: &str,
    reader: &mut R,
    git_hash: HashSums,
) -> Result<bool>
```

### `utils` - Utilities

Helper functions for common operations.

**Key Functions:**
```rust
// Create temp directory for downloads (with disk space checking)
pub fn create_download_temp_dir(version: &str, download_url: &str) -> std::io::Result<PathBuf>

// Get system temp directory (with fallback)
pub fn get_temp_dir() -> std::io::Result<PathBuf>

// Clean up fallback temp directory
pub fn cleanup_fallback_temp_dir() -> std::io::Result<()>

// Expand tilde (~) in paths
pub fn expand_tilde<P: AsRef<Path>>(path: &Path) -> Option<PathBuf>

// Match version string against release tag
pub fn match_version(version_str: &str, tag_name: &str) -> bool
```

### `constants` - Configuration Constants

```rust
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_STEAM_TOOL: &str = "GEProton";
pub const DEFAULT_LUTRIS_TOOL: &str = "WineGE";
pub const USER_AGENT: &str = "protonup-rs";
pub const MIN_TEMP_SPACE_BYTES: u64 = 1_073_741_824; // 1GB
pub const FALLBACK_TEMP_DIR: &str = ".local/state/protonup-rs/tmp";
```

## Usage Patterns

### Quick Update (run_quick_downloads)

The primary workflow for the "Quick Update" feature:

```rust
use libprotonup::{
    apps::list_installed_apps,
    downloads::{self, Release},
    sources::CompatTool,
};

// 1. Detect installed apps
let found_apps = list_installed_apps().await;

// 2. For each app, get default compatibility tool
for app_inst in found_apps {
    let compat_tool = app_inst.as_app().default_compatibility_tool();
    
    // 3. Fetch latest release
    let releases = downloads::list_releases(&compat_tool).await?;
    let latest_release = &releases[0];
    
    // 4. Get download info
    let download = latest_release.get_download_info(&app_inst, &compat_tool);
    
    // 5. Download, validate, and unpack
    // (See protonup-rs/src/download.rs for full implementation)
}
```

### Download to Selected App

For user-directed installations:

```rust
use libprotonup::{
    apps::{App, AppInstallations},
    downloads,
    sources::{CompatTool, CompatTools},
};

// 1. Get available tools for app
let available_tools = CompatTool::sources_for_app(&App::Steam);

// 2. User selects tool
let selected_tool = available_tools[0].clone();

// 3. Fetch releases
let releases = downloads::list_releases(&selected_tool).await?;

// 4. User selects version(s)
let selected_release = &releases[0];

// 5. Get download info (handle architecture variants if needed)
let download = if selected_tool.has_multiple_asset_variations {
    let variants = selected_release.get_all_download_variants(&app_inst, &selected_tool);
    // Select appropriate variant
    variants[0].clone()
} else {
    selected_release.get_download_info(&app_inst, &selected_tool)
};

// 6. Download, validate, unpack
```

## Installation Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. DETECT APPS                                                   │
│    list_installed_apps() → [Steam, Lutris]                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. GET COMPAT TOOL                                              │
│    app.default_compatibility_tool() → GEProton                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. FETCH RELEASES                                               │
│    downloads::list_releases(&tool) → [Release]                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. GET DOWNLOAD INFO                                            │
│    release.get_download_info(&app, &tool) → Download            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. CREATE TEMP DIR                                              │
│    utils::create_download_temp_dir() → PathBuf                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 6. DOWNLOAD FILE                                                │
│    downloads::download_to_async_write()                         │
│    (wrap with progress reporter)                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 7. VERIFY HASH (if available)                                   │
│    hashing::hash_check_file() → bool                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 8. UNPACK ARCHIVE                                               │
│    files::unpack_file()                                         │
│    - Creates decompressor (Gzip/Xz/Zstd)                        │
│    - Extracts to install_dir                                    │
│    - Replaces top-level folder with tool name                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 9. CLEANUP                                                      │
│    utils::cleanup_fallback_temp_dir()                           │
└─────────────────────────────────────────────────────────────────┘
```

## Progress Reporting

For GUI integration, wrap async readers/writers with progress reporters:

```rust
// Download progress
let progress_bar = ProgressBar::new(download.size);
downloads::download_to_async_write(
    &download.download_url,
    &mut progress_bar.wrap_async_write(file),
).await?;

// Hash verification progress
let hash_bar = ProgressBar::new(fs::metadata(&path).await?.len());
hashing::hash_check_file(
    &download.file_name,
    &mut hash_bar.wrap_async_read(BufReader::new(file)),
    hash_sum,
).await?;

// Unpack progress
let unpack_bar = ProgressBar::new(fs::metadata(&file).await?.len());
files::unpack_file(
    &compat_tool,
    &download,
    unpack_bar.wrap_async_read(decompressor),
    &install_dir,
).await?;
```

## Error Handling

All functions return `Result<T, Error>` types:
- `anyhow::Error` - General errors with context
- `reqwest::Error` - Network errors
- `io::Error` - File system errors

Use `.with_context()` to add meaningful error messages.

## Testing

The library includes comprehensive tests:
- Unit tests for each module
- Mocked GitHub API tests
- Integration tests for unpacking
- Version matching tests

Run tests with:
```bash
cargo test -p libprotonup
```

## Supported Tools

See `sources.ron` for the complete list. Currently supported:

| Tool | Repository | Compatible With |
|------|------------|-----------------|
| GEProton | GloriousEggroll/proton-ge-custom | Steam |
| WineGE | GloriousEggroll/wine-ge-custom | Lutris |
| Luxtorpeda | luxtorpeda-dev/luxtorpeda | Steam |
| Boxtron | dreamer/boxtron | Steam |
| VKD3D-Proton | HansKristian-Work/vkd3d-proton | Steam, Lutris |
| Lutris-VKD3D | lutris/vkd3d | Lutris |
| DXVK | doitsujin/dxvk | Steam, Lutris |
| Kron4ek Wine | kron4ek/Wine-Builds | Lutris |
| GEProton-rtsp | SpookySkeletons/proton-ge-rtsp | Steam |
| Proton-CachyOS | CachyOS/proton-cachyos | Steam |

## GUI Implementation Notes

For implementing the GUI, focus on these key integration points:

1. **App Detection**: Call `list_installed_apps()` on startup
2. **Release Fetching**: Use `downloads::list_releases()` to get available versions
3. **Download Streaming**: Use `Task::sip()` pattern with `download_to_async_write()`
4. **Progress Reporting**: Wrap async operations with progress callbacks
5. **Multi-download**: Use `tokio::spawn` for parallel downloads (like `run_quick_downloads`)

See `protonup-rs/src/download.rs` for the complete reference implementation of the download-validate-unpack flow.
