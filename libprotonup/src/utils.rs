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

/// Matches a user-provided version string against a release tag name.
/// Handles various version formats like "GE-Proton8-26", "8.7-GE-1-LoL", etc.
///
/// Matching strategies (in order):
/// 1. Exact match
/// 2. Prefix match (tag starts with user input)
/// 3. Component match - extract numeric components and check if user's components appear in tag
pub fn match_version(version_str: &str, tag_name: &str) -> bool {
    // Reject empty input
    if version_str.is_empty() {
        return false;
    }

    // Exact match
    if tag_name == version_str {
        return true;
    }

    // Prefix match
    if tag_name.starts_with(version_str) {
        return true;
    }

    // Extract numeric components from user input (e.g., "10.6" -> ["10", "6"])
    let user_components: Vec<&str> = version_str
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .collect();

    if user_components.is_empty() {
        return false;
    }

    // Extract numeric components from tag name
    let tag_components: Vec<&str> = tag_name
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .collect();

    // Check if user's components appear in sequence in the tag's components
    // e.g., user "8.26" should match "GE-Proton8-26"
    if tag_components
        .windows(user_components.len())
        .any(|window| window == user_components.as_slice())
    {
        return true;
    }

    // Also try matching just the major version if user provided multiple components
    // e.g., user "10" should match "GE-Proton10-6"
    if user_components.len() == 1 {
        return tag_components.contains(&user_components[0]);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test cases for match_version function
    // Format: (user_input, tag_name, expected_match)
    const MATCH_TEST_CASES: &[(&str, &str, bool)] = &[
        // --- Exact matches ---
        ("v2.7.1", "v2.7.1", true),
        ("v2.14", "v2.14", true),
        ("v0.5.4", "v0.5.4", true),
        //
        // --- CachyOS style: cachyos-10.0-20260228-slr ---
        ("10.0", "cachyos-10.0-20260228-slr", true),
        ("10", "cachyos-10.0-20260228-slr", true),
        ("20260228", "cachyos-10.0-20260228-slr", true),
        ("10.1", "cachyos-10.0-20260228-slr", false),
        //
        // --- GE-Proton style: GE-Proton10-26-rtsp20 ---
        ("10.26", "GE-Proton10-26-rtsp20", true),
        ("10", "GE-Proton10-26-rtsp20", true),
        ("26", "GE-Proton10-26-rtsp20", true),
        ("20", "GE-Proton10-26-rtsp20", true), // rtsp20 contains 20
        ("10.27", "GE-Proton10-26-rtsp20", false),
        //
        // --- GE-Proton standard: GE-Proton8-26, GE-Proton10-32 ---
        ("8.26", "GE-Proton8-26", true),
        ("8", "GE-Proton8-26", true),
        ("26", "GE-Proton8-26", true),
        ("10.32", "GE-Proton10-32", true),
        ("10", "GE-Proton10-32", true),
        ("32", "GE-Proton10-32", true),
        ("9.26", "GE-Proton8-26", false),
        //
        // --- Simple v-prefixed versions ---
        ("2.7.1", "v2.7.1", true),
        ("2.7", "v2.7.1", true), // First two components match
        ("2", "v2.7.1", true),
        ("2.14", "v2.14", true),
        ("2", "v2.14", true),
        ("0.5.4", "v0.5.4", true),
        ("0.5", "v0.5.4", true),
        ("3.0", "v3.0b", true), // Components match, ignores trailing 'b'
        ("3", "v3.0b", true),
        ("76.2.0", "v76.2.0", true),
        ("76.2", "v76.2.0", true),
        ("76", "v76.2.0", true),
        //
        // --- Edge cases ---
        ("", "v2.7.1", false),      // Empty input
        ("abc", "v2.7.1", false),   // Non-numeric input
        ("2.7.1", "v2.7.2", false), // Different patch version
        ("2.8", "v2.7.1", false),   // Different minor version
        ("3", "v2.7.1", false),     // Different major version
        //
        // --- Prefix matching ---
        ("v2", "v2.7.1", true),
        ("GE-Proton8", "GE-Proton8-26", true),
        ("cachyos", "cachyos-10.0-20260228-slr", true),
    ];

    #[test]
    fn test_match_version() {
        for (user_input, tag_name, expected) in MATCH_TEST_CASES {
            let result = match_version(user_input, tag_name);
            assert_eq!(
                result, *expected,
                "match_version(\"{}\", \"{}\") - expected {}, got {}",
                user_input, tag_name, expected, result
            );
        }
    }

    #[test]
    fn test_match_version_geproton_variants() {
        // Test GE-Proton specific patterns
        assert!(match_version("8-26", "GE-Proton8-26"));
        assert!(match_version("10-32", "GE-Proton10-32"));
        assert!(!match_version("8-27", "GE-Proton8-26"));
        //
        // Test with LoL variants
        assert!(match_version("8-27", "GE-Proton8-27-LoL"));
        assert!(match_version("8.27", "GE-Proton8-27-LoL"));
    }

    #[test]
    fn test_match_version_component_extraction() {
        // Test that numeric components are correctly extracted and matched
        assert!(match_version("1.2.3", "tool-1.2.3-release"));
        assert!(match_version("1.2", "tool-1.2.3-release"));
        assert!(match_version("1", "tool-1.2.3-release"));
        assert!(!match_version("2.3.4", "tool-1.2.3-release")); // Different components
        assert!(!match_version("4.5.6", "tool-1.2.3-release")); // Completely different
    }

    #[test]
    fn test_match_version_prefix_fallback() {
        // Test prefix matching as fallback
        assert!(match_version("GE-Proton", "GE-Proton8-26"));
        assert!(match_version("v2", "v2.7.1"));
        assert!(match_version("cachyos-", "cachyos-10.0-20260228-slr"));
    }

    #[test]
    fn test_match_version_consecutive_components() {
        // Test matching consecutive numeric components
        assert!(match_version("10.0", "cachyos-10.0-20260228-slr"));
        assert!(match_version("10.26", "GE-Proton10-26-rtsp20"));
        assert!(!match_version("10.27", "GE-Proton10-26-rtsp20"));
        // Note: "26.20" matches because tag components are [GE, Proton, 10, 26, rtsp, 20]
        // and [26, 20] are indeed consecutive in this list (after rtsp is split)
        // This is expected behavior - user input "26.20" will match tags containing those
        // consecutive numeric components
        assert!(match_version("26.20", "GE-Proton10-26-rtsp20"));
    }
}
