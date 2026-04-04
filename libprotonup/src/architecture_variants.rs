//! Architecture variant detection for Proton CachyOS and similar tools.
//!
//! This module provides functions to detect, extract, and select CPU architecture
//! variants from download files (e.g., x86_64, x86_64_v2, x86_64_v3, x86_64_v4).

use crate::downloads::Download;

/// Architecture variant for Proton CachyOS and similar tools.
///
/// Contains the variant name, description, and download information.
#[derive(Debug, Clone)]
pub struct ArchitectureVariant {
    /// The architecture variant name (x86_64, x86_64_v2, x86_64_v3, x86_64_v4)
    pub name: String,
    /// Extended description of this variant
    pub description: String,
    /// The download information for this variant
    pub download: Download,
}

impl std::fmt::Display for ArchitectureVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {}", self.name, self.description)
    }
}

/// Extracts the architecture variant code from a file name.
///
/// Returns:
/// - `1`: x86_64 (universal)
/// - `2`: x86_64_v2
/// - `3`: x86_64_v3
/// - `4`: x86_64_v4
/// - `0`: unknown/not a recognized variant
pub fn get_architecture_variant(file_name: &str) -> u8 {
    if file_name.contains("_v4") {
        4
    } else if file_name.contains("_v3") {
        3
    } else if file_name.contains("_v2") {
        2
    } else if file_name.contains("-x86_64.") {
        1
    } else {
        0
    }
}

/// Gets the variant name string from the variant code.
pub fn get_variant_name(variant_code: u8) -> &'static str {
    match variant_code {
        1 => "x86_64",
        2 => "x86_64_v2",
        3 => "x86_64_v3",
        4 => "x86_64_v4",
        _ => "unknown",
    }
}

/// Gets an extended description for an architecture variant.
pub fn get_architecture_description(variant_code: u8) -> &'static str {
    match variant_code {
        4 => "Experimental - optimized for AVX-512",
        3 => "Modern CPUs - optimized for AVX2",
        2 => "Recommended - optimized for SSE3",
        1 => "Universal - all x86-64 CPUs",
        _ => "Unknown",
    }
}

/// Extracts architecture variants from a list of downloads.
///
/// Returns a sorted Vec<ArchitectureVariant> sorted by variant priority
/// (x86_64 < x86_64_v2 < x86_64_v3 < x86_64_v4).
pub fn extract_variants(downloads: &[Download]) -> Vec<ArchitectureVariant> {
    let mut variants: Vec<ArchitectureVariant> = downloads
        .iter()
        .filter_map(|download| {
            let variant_code = get_architecture_variant(&download.file_name);
            if variant_code == 0 {
                return None;
            }
            Some(ArchitectureVariant {
                name: get_variant_name(variant_code).to_string(),
                description: get_architecture_description(variant_code).to_string(),
                download: download.clone(),
            })
        })
        .collect();

    // Sort by variant priority (x86_64 < v2 < v3 < v4)
    variants.sort_by_key(|v| {
        match v.name.as_str() {
            "x86_64" => 1,
            "x86_64_v2" => 2,
            "x86_64_v3" => 3,
            "x86_64_v4" => 4,
            _ => 99,
        }
    });

    variants
}

/// Selects the default variant for quick mode.
///
/// Prefers `_v2` variant if available, otherwise falls back to the first available variant.
/// Returns `None` if no variants are available.
pub fn select_default_variant(downloads: &[Download]) -> Option<Download> {
    downloads
        .iter()
        .find(|d| d.file_name.contains("_v2"))
        .or_else(|| downloads.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_architecture_variant() {
        assert_eq!(get_architecture_variant("proton-cachyos-x86_64.tar.gz"), 1);
        assert_eq!(get_architecture_variant("proton-cachyos-x86_64_v2.tar.gz"), 2);
        assert_eq!(get_architecture_variant("proton-cachyos-x86_64_v3.tar.gz"), 3);
        assert_eq!(get_architecture_variant("proton-cachyos-x86_64_v4.tar.gz"), 4);
        assert_eq!(get_architecture_variant("some-other-file.tar.gz"), 0);
    }

    #[test]
    fn test_get_variant_name() {
        assert_eq!(get_variant_name(1), "x86_64");
        assert_eq!(get_variant_name(2), "x86_64_v2");
        assert_eq!(get_variant_name(3), "x86_64_v3");
        assert_eq!(get_variant_name(4), "x86_64_v4");
        assert_eq!(get_variant_name(0), "unknown");
    }

    #[test]
    fn test_get_architecture_description() {
        assert!(get_architecture_description(1).contains("Universal"));
        assert!(get_architecture_description(2).contains("SSE3"));
        assert!(get_architecture_description(3).contains("AVX2"));
        assert!(get_architecture_description(4).contains("AVX-512"));
    }

    #[test]
    fn test_extract_variants_sorts_correctly() {
        let downloads = vec![
            create_mock_download("proton-x86_64_v3.tar.gz"),
            create_mock_download("proton-x86_64.tar.gz"),
            create_mock_download("proton-x86_64_v4.tar.gz"),
            create_mock_download("proton-x86_64_v2.tar.gz"),
        ];

        let variants = extract_variants(&downloads);

        assert_eq!(variants.len(), 4);
        assert_eq!(variants[0].name, "x86_64");
        assert_eq!(variants[1].name, "x86_64_v2");
        assert_eq!(variants[2].name, "x86_64_v3");
        assert_eq!(variants[3].name, "x86_64_v4");
    }

    #[test]
    fn test_extract_variants_filters_unknown() {
        let downloads = vec![
            create_mock_download("proton-x86_64_v2.tar.gz"),
            create_mock_download("some-other-file.tar.gz"),
        ];

        let variants = extract_variants(&downloads);

        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "x86_64_v2");
    }

    #[test]
    fn test_select_default_variant_prefers_v2() {
        let downloads = vec![
            create_mock_download("proton-x86_64.tar.gz"),
            create_mock_download("proton-x86_64_v2.tar.gz"),
            create_mock_download("proton-x86_64_v3.tar.gz"),
        ];

        let default = select_default_variant(&downloads).unwrap();
        assert!(default.file_name.contains("_v2"));
    }

    #[test]
    fn test_select_default_variant_falls_back_to_first() {
        let downloads = vec![
            create_mock_download("proton-x86_64_v3.tar.gz"),
            create_mock_download("proton-x86_64_v4.tar.gz"),
        ];

        let default = select_default_variant(&downloads).unwrap();
        assert!(default.file_name.contains("x86_64_v3"));
    }

    #[test]
    fn test_select_default_variant_returns_none_for_empty() {
        let downloads: Vec<Download> = vec![];
        assert!(select_default_variant(&downloads).is_none());
    }

    fn create_mock_download(file_name: &str) -> Download {
        Download {
            file_name: file_name.to_string(),
            for_app: crate::apps::AppInstallations::Steam,
            version: "test".to_string(),
            hash_sum: None,
            download_url: "https://example.com/test".to_string(),
            size: 1000,
        }
    }
}
