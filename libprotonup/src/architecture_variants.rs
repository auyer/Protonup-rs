//! Architecture variant detection for Proton CachyOS and similar tools.
//!
//! This module provides functions to detect, extract, and select CPU architecture
//! variants from download files (e.g., x86_64, x86_64_v2, x86_64_v3, x86_64_v4).

use crate::downloads::Download;

/// Architecture variant for Proton CachyOS and similar tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MicroArchVariants {
    X86_64,
    X86_64V2,
    X86_64V3,
    X86_64V4,
}

impl MicroArchVariants {
    /// The architecture variant name as it appears in file names.
    pub fn name(&self) -> &'static str {
        match self {
            MicroArchVariants::X86_64 => "x86_64",
            MicroArchVariants::X86_64V2 => "x86_64_v2",
            MicroArchVariants::X86_64V3 => "x86_64_v3",
            MicroArchVariants::X86_64V4 => "x86_64_v4",
        }
    }

    /// Extended description of this variant.
    pub fn description(&self) -> &'static str {
        match self {
            MicroArchVariants::X86_64 => "Universal - all x86-64 CPUs",
            MicroArchVariants::X86_64V2 => "Recommended - optimized for SSE3",
            MicroArchVariants::X86_64V3 => "Modern CPUs - optimized for AVX2",
            MicroArchVariants::X86_64V4 => "Experimental - optimized for AVX-512",
        }
    }

    /// Detects the architecture variant from a file name.
    ///
    /// Returns `None` if the file name does not match a recognized variant.
    pub fn from_file_name(file_name: &str) -> Option<Self> {
        if file_name.contains("_v4") {
            Some(MicroArchVariants::X86_64V4)
        } else if file_name.contains("_v3") {
            Some(MicroArchVariants::X86_64V3)
        } else if file_name.contains("_v2") {
            Some(MicroArchVariants::X86_64V2)
        } else if file_name.contains("-x86_64.") {
            Some(MicroArchVariants::X86_64)
        } else {
            None
        }
    }
}

/// Details for a micro architecture variant.
#[derive(Debug, Clone)]
pub struct MicroArchDetails {
    /// The detected architecture variant
    pub variant: MicroArchVariants,
    /// The download information for this variant
    pub download: Download,
}

impl MicroArchDetails {
    /// The architecture variant name as it appears in file names.
    pub fn name(&self) -> &'static str {
        self.variant.name()
    }

    /// Extended description of this variant.
    pub fn description(&self) -> &'static str {
        self.variant.description()
    }
}

impl std::fmt::Display for MicroArchDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {}", self.name(), self.description())
    }
}

/// Extracts micro architecture variants from a list of downloads.
///
/// Returns a sorted Vec<MicroArchDetails> sorted by variant priority
/// (x86_64 < x86_64_v2 < x86_64_v3 < x86_64_v4).
pub fn extract_march_variants(downloads: &[Download]) -> Vec<MicroArchDetails> {
    let mut variants: Vec<MicroArchDetails> = downloads
        .iter()
        .filter_map(|download| {
            let variant = MicroArchVariants::from_file_name(&download.file_name)?;
            Some(MicroArchDetails {
                variant,
                download: download.clone(),
            })
        })
        .collect();

    variants.sort_by_key(|v| v.variant);

    variants
}

/// Selects the default variant for quick mode.
///
/// Prefers the `x86_64_v2` variant if available, otherwise falls back to the
/// first available variant.
/// Returns `None` if no variants are available.
pub fn select_default_variant(downloads: &[Download]) -> Option<Download> {
    downloads
        .iter()
        .find(|d| {
            MicroArchVariants::from_file_name(&d.file_name) == Some(MicroArchVariants::X86_64V2)
        })
        .or_else(|| downloads.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_file_name() {
        assert_eq!(
            MicroArchVariants::from_file_name("proton-cachyos-x86_64.tar.gz"),
            Some(MicroArchVariants::X86_64)
        );
        assert_eq!(
            MicroArchVariants::from_file_name("proton-cachyos-x86_64_v2.tar.gz"),
            Some(MicroArchVariants::X86_64V2)
        );
        assert_eq!(
            MicroArchVariants::from_file_name("proton-cachyos-x86_64_v3.tar.gz"),
            Some(MicroArchVariants::X86_64V3)
        );
        assert_eq!(
            MicroArchVariants::from_file_name("proton-cachyos-x86_64_v4.tar.gz"),
            Some(MicroArchVariants::X86_64V4)
        );
        assert_eq!(
            MicroArchVariants::from_file_name("some-other-file.tar.gz"),
            None
        );
    }

    #[test]
    fn test_name() {
        assert_eq!(MicroArchVariants::X86_64.name(), "x86_64");
        assert_eq!(MicroArchVariants::X86_64V2.name(), "x86_64_v2");
        assert_eq!(MicroArchVariants::X86_64V3.name(), "x86_64_v3");
        assert_eq!(MicroArchVariants::X86_64V4.name(), "x86_64_v4");
    }

    #[test]
    fn test_description() {
        assert!(
            MicroArchVariants::X86_64
                .description()
                .contains("Universal")
        );
        assert!(MicroArchVariants::X86_64V2.description().contains("SSE3"));
        assert!(MicroArchVariants::X86_64V3.description().contains("AVX2"));
        assert!(
            MicroArchVariants::X86_64V4
                .description()
                .contains("AVX-512")
        );
    }

    #[test]
    fn test_extract_march_variants_sorts_correctly() {
        let downloads = vec![
            create_mock_download("proton-x86_64_v3.tar.gz"),
            create_mock_download("proton-x86_64.tar.gz"),
            create_mock_download("proton-x86_64_v4.tar.gz"),
            create_mock_download("proton-x86_64_v2.tar.gz"),
        ];

        let variants = extract_march_variants(&downloads);

        assert_eq!(variants.len(), 4);
        assert_eq!(variants[0].variant, MicroArchVariants::X86_64);
        assert_eq!(variants[1].variant, MicroArchVariants::X86_64V2);
        assert_eq!(variants[2].variant, MicroArchVariants::X86_64V3);
        assert_eq!(variants[3].variant, MicroArchVariants::X86_64V4);
    }

    #[test]
    fn test_extract_march_variants_filters_unknown() {
        let downloads = vec![
            create_mock_download("proton-x86_64_v2.tar.gz"),
            create_mock_download("some-other-file.tar.gz"),
        ];

        let variants = extract_march_variants(&downloads);

        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].variant, MicroArchVariants::X86_64V2);
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
