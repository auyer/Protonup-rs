use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::constants;

/// Builds the fallback temp directory path from the constant.
/// The FALLBACK_TEMP_DIR constant contains the full relative path (e.g., ".local/state/protonup-rs/tmp")
/// This function splits it by '/' and joins each component to the base directory.
fn build_fallback_dir(base_dir: &Path) -> PathBuf {
    constants::FALLBACK_TEMP_DIR
        .split('/')
        .fold(base_dir.to_path_buf(), |acc, component| acc.join(component))
}

/// Checks available disk space and returns an appropriate temp directory.
/// If /tmp has less than 1GB available, returns a fallback directory under
/// .local/state/protonup-rs/tmp. Creates the fallback directory if it doesn't exist.
pub fn get_temp_dir() -> std::io::Result<PathBuf> {
    // Check available space on tmp dir
    let tmp_path = TempDir::with_prefix("protonup-rs-").map(|dir| dir.keep());

    if let Ok(temp_dir) = tmp_path {
        let available_space = fs4::available_space(&temp_dir).unwrap_or(0);

        if available_space >= constants::MIN_TEMP_SPACE_BYTES {
            // Enough space, use standard tempdir
            return Ok(temp_dir);
        }
    }

    // Not enough space, or failed creating the temp dir, use fallback directory
    let fallback_dir = match dirs::state_dir() {
        Some(state_dir) => build_fallback_dir(&state_dir),
        None => {
            // Fallback to home directory if state_dir is not available
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            build_fallback_dir(&home_dir)
        }
    };

    // Create the fallback directory if it doesn't exist
    fs::create_dir_all(&fallback_dir)?;

    Ok(fallback_dir)
}

/// Creates a temporary download directory for a specific version.
/// Handles disk space checking and creates the necessary directory structure.
/// Returns the full path where the download file should be saved.
pub fn create_download_temp_dir(version: &str, download_url: &str) -> std::io::Result<PathBuf> {
    let temp_dir = get_temp_dir()?;

    let download_dir = temp_dir.join(format!("{}.{}", version, "download"));

    // Create the version-specific subdirectory
    fs::create_dir_all(&download_dir)?;

    // Determine the file extension from the download URL
    let ext = crate::files::check_supported_extension(download_url)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

    let download_path = download_dir.join(format!("{}.{}", version, ext));

    Ok(download_path)
}

/// Cleans up the fallback temp directory contents.
/// This should be called on application exit to remove temporary files.
pub fn cleanup_fallback_temp_dir() -> std::io::Result<()> {
    let fallback_dir = match dirs::state_dir() {
        Some(state_dir) => build_fallback_dir(&state_dir),
        None => {
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            build_fallback_dir(&home_dir)
        }
    };

    if fallback_dir.exists() {
        // Remove all contents of the fallback directory
        for entry in fs::read_dir(&fallback_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}

pub fn expand_tilde<P: AsRef<Path>>(path_user_input: P) -> Option<PathBuf> {
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
