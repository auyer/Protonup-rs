//! Tool variant detection.
//!
//! Releases of some tools ship several archives for the same version, each
//! targeting a different build flavour: CPU micro-architecture levels
//! (`x86_64_v2`, `x86_64_v3`, ...), a WoW64-only build, or a native build
//! (`dxvk-native-*`).
//!
//! This module maps an archive file name to a *known* [`ToolVariant`] when we
//! recognise the flavour, and otherwise falls back to
//! [`ToolVariantChoice::Opaque`], which simply carries the original file name so
//! unknown variations can still be offered to the user unchanged.
//!
//! Detection deliberately ignores version numbering: only stable tokens such as
//! `_v3`, `_wow64` or `-native` are inspected.

use crate::downloads::Download;

/// A build flavour of a release we know how to label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolVariant {
    /// Generic x86-64 build (`x86_64`).
    X86_64,
    /// x86-64 with the v2 micro-architecture baseline.
    X86_64V2,
    /// x86-64 with the v3 micro-architecture baseline.
    X86_64V3,
    /// x86-64 with the v4 micro-architecture baseline.
    X86_64V4,
    /// Windows 64-bit only mode build.
    Wow64,
    /// Native (Linux) build, e.g. `dxvk-native-*`.
    Native,
}

impl ToolVariant {
    /// Every known variant, in display/priority order.
    pub fn all() -> &'static [ToolVariant] {
        &[
            ToolVariant::X86_64,
            ToolVariant::X86_64V2,
            ToolVariant::X86_64V3,
            ToolVariant::X86_64V4,
            ToolVariant::Wow64,
            ToolVariant::Native,
        ]
    }

    /// The token as it appears in file names.
    pub fn name(&self) -> &'static str {
        match self {
            ToolVariant::X86_64 => "x86_64",
            ToolVariant::X86_64V2 => "x86_64_v2",
            ToolVariant::X86_64V3 => "x86_64_v3",
            ToolVariant::X86_64V4 => "x86_64_v4",
            ToolVariant::Wow64 => "wow64",
            ToolVariant::Native => "native",
        }
    }

    /// Extended description of this variant.
    pub fn description(&self) -> &'static str {
        match self {
            ToolVariant::X86_64 => "Universal - all x86-64 CPUs",
            ToolVariant::X86_64V2 => "Recommended - optimized for SSE3",
            ToolVariant::X86_64V3 => "Modern CPUs - optimized for AVX2",
            ToolVariant::X86_64V4 => "Experimental - optimized for AVX-512",
            ToolVariant::Wow64 => "64-bit only",
            ToolVariant::Native => "Linux native library",
        }
    }

    /// Human readable label used in menus, e.g. `WoW64` or `Native`.
    pub fn display_name(&self) -> &'static str {
        match self {
            ToolVariant::X86_64 => "x86_64",
            ToolVariant::X86_64V2 => "x86_64_v2",
            ToolVariant::X86_64V3 => "x86_64_v3",
            ToolVariant::X86_64V4 => "x86_64_v4",
            ToolVariant::Wow64 => "WoW64",
            ToolVariant::Native => "Native",
        }
    }

    /// Detects a known variant from a file name.
    ///
    /// Returns `None` when the name carries no recognised flavour token.
    /// The most specific tokens are checked first, so `x86_64_v3` wins over the
    /// plain `x86_64` marker.
    pub fn from_file_name(file_name: &str) -> Option<Self> {
        let lower = file_name.to_ascii_lowercase();

        if contains_token(&lower, "_v4") {
            Some(ToolVariant::X86_64V4)
        } else if contains_token(&lower, "_v3") {
            Some(ToolVariant::X86_64V3)
        } else if contains_token(&lower, "_v2") {
            Some(ToolVariant::X86_64V2)
        } else if contains_token(&lower, "_wow64") || contains_token(&lower, "-wow64") {
            Some(ToolVariant::Wow64)
        } else if contains_token(&lower, "-native") {
            Some(ToolVariant::Native)
        } else if lower.contains("-x86_64.") || lower.contains("-x86-64.") {
            Some(ToolVariant::X86_64)
        } else {
            None
        }
    }
}

/// Returns true when `token` appears in `name` and is not part of a longer
/// digit/letter run, i.e. it is delimited by `-`, `_`, `.` or a string edge.
///
/// This keeps `_v3` from matching inside `_v30` while still matching the
/// trailing `_v3` of `proton-x86_64_v3.tar.xz`.
fn contains_token(name: &str, token: &str) -> bool {
    let mut start = 0;
    while let Some(rel) = name[start..].find(token) {
        let abs = start + rel;
        let after = abs + token.len();

        // The character right after the token must not continue a version-ish
        // run (e.g. a digit), otherwise `_v3` would match `_v30`.
        let after_ok = match name[after..].chars().next() {
            None => true,
            Some(c) => !c.is_ascii_digit(),
        };

        if after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// Result of matching a download against the known tool variants.
///
/// Known variants get a descriptive label; anything else keeps its original
/// file name so unknown variations are still offered to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolVariantChoice {
    /// A recognised build flavour.
    Known(ToolVariant),
    /// An unrecognised variation, identified by its archive file name.
    Opaque(String),
}

impl ToolVariantChoice {
    /// Matches `file_name` against the known variants, falling back to
    /// [`ToolVariantChoice::Opaque`] with the original name.
    pub fn from_file_name(file_name: &str) -> Self {
        match ToolVariant::from_file_name(file_name) {
            Some(variant) => ToolVariantChoice::Known(variant),
            None => ToolVariantChoice::Opaque(file_name.to_owned()),
        }
    }

    /// Label shown to the user, e.g. `Modern CPUs - optimized for AVX2` or the
    /// raw file name for opaque variations.
    pub fn label(&self) -> String {
        match self {
            ToolVariantChoice::Known(variant) => {
                format!("{} - {}", variant.display_name(), variant.description())
            }
            ToolVariantChoice::Opaque(name) => name.clone(),
        }
    }

    /// Sort key: known variants follow the registry order, opaque entries sort
    /// after every known variant.
    fn sort_key(&self) -> (u8, usize) {
        match self {
            ToolVariantChoice::Known(variant) => (
                0,
                ToolVariant::all()
                    .iter()
                    .position(|v| v == variant)
                    .unwrap_or(usize::MAX),
            ),
            ToolVariantChoice::Opaque(_) => (1, 0),
        }
    }
}

impl std::fmt::Display for ToolVariantChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Details for a tool variant: the matched choice and its download.
#[derive(Debug, Clone)]
pub struct ToolVariantDetails {
    /// The matched variant (known or opaque).
    pub choice: ToolVariantChoice,
    /// The download information for this variant.
    pub download: Download,
}

impl ToolVariantDetails {
    /// Label shown to the user.
    pub fn label(&self) -> String {
        self.choice.label()
    }

    /// The archive file name.
    pub fn file_name(&self) -> &str {
        &self.download.file_name
    }
}

impl std::fmt::Display for ToolVariantDetails {
    /// Renders as `<label> (<file name>)`, which is what the TUI menu shows.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({})", self.label(), self.file_name())
    }
}

/// Builds the list of variants for a release.
///
/// Known variants come first, in registry order, followed by opaque
/// (unrecognised) variations in their original asset order. Entries with a
/// duplicated file name are dropped.
pub fn extract_tool_variants(downloads: &[Download]) -> Vec<ToolVariantDetails> {
    let mut variants: Vec<ToolVariantDetails> = Vec::new();

    for download in downloads {
        let choice = ToolVariantChoice::from_file_name(&download.file_name);
        let details = ToolVariantDetails {
            choice,
            download: download.clone(),
        };
        if variants
            .iter()
            .any(|existing| existing.file_name() == details.file_name())
        {
            continue;
        }
        variants.push(details);
    }

    // Stable ordering: known variants by registry order, opaque ones afterwards
    // in asset order.
    variants.sort_by_key(|v| v.choice.sort_key());

    variants
}

/// Selects the default variant for quick mode.
///
/// Prefers the `x86_64_v2` variant when available, otherwise falls back to the
/// first available variant. Returns `None` if no variants are available.
pub fn select_default_variant(downloads: &[Download]) -> Option<Download> {
    downloads
        .iter()
        .find(|d| ToolVariant::from_file_name(&d.file_name) == Some(ToolVariant::X86_64V2))
        .or_else(|| downloads.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_file_name_microarch() {
        assert_eq!(
            ToolVariant::from_file_name("proton-cachyos-11.0-x86_64.tar.xz"),
            Some(ToolVariant::X86_64)
        );
        assert_eq!(
            ToolVariant::from_file_name("proton-cachyos-11.0-x86_64_v2.tar.xz"),
            Some(ToolVariant::X86_64V2)
        );
        assert_eq!(
            ToolVariant::from_file_name("proton-cachyos-11.0-x86_64_v3.tar.xz"),
            Some(ToolVariant::X86_64V3)
        );
        assert_eq!(
            ToolVariant::from_file_name("proton-cachyos-11.0-x86_64_v4.tar.xz"),
            Some(ToolVariant::X86_64V4)
        );
    }

    #[test]
    fn test_from_file_name_wow64() {
        assert_eq!(
            ToolVariant::from_file_name("proton-wineland-11.0-20261106-x86_64_wow64.tar.xz"),
            Some(ToolVariant::Wow64)
        );
        assert_eq!(
            ToolVariant::from_file_name("wine-11.21-amd64-wow64.tar.xz"),
            Some(ToolVariant::Wow64)
        );
    }

    #[test]
    fn test_from_file_name_native() {
        assert_eq!(
            ToolVariant::from_file_name("dxvk-native-3.1.1-steamrt-sniper.tar.gz"),
            Some(ToolVariant::Native)
        );
    }

    #[test]
    fn test_from_file_name_does_not_match_longer_version_run() {
        // `_v30` must not be seen as `_v3`.
        assert_eq!(ToolVariant::from_file_name("app-x86_64_v30.tar.gz"), None);
    }

    #[test]
    fn test_from_file_name_plain_dxvk_is_unknown() {
        assert_eq!(ToolVariant::from_file_name("dxvk-3.1.1.tar.gz"), None);
        assert_eq!(
            ToolVariantChoice::from_file_name("dxvk-3.1.1.tar.gz"),
            ToolVariantChoice::Opaque("dxvk-3.1.1.tar.gz".to_string())
        );
    }

    #[test]
    fn test_choice_label_known() {
        let choice = ToolVariantChoice::from_file_name("proton-x86_64_v3.tar.xz");
        assert_eq!(
            choice.label(),
            "x86_64_v3 - Modern CPUs - optimized for AVX2"
        );
    }

    #[test]
    fn test_choice_label_opaque_is_file_name() {
        let choice = ToolVariantChoice::from_file_name("dxvk-3.1.1.tar.gz");
        assert_eq!(
            choice,
            ToolVariantChoice::Opaque("dxvk-3.1.1.tar.gz".into())
        );
        assert_eq!(choice.label(), "dxvk-3.1.1.tar.gz");
    }

    #[test]
    fn test_extract_tool_variants_sorts_known_first() {
        let downloads = vec![
            create_mock_download("dxvk-native-3.1.1-steamrt-sniper.tar.gz"),
            create_mock_download("dxvk-3.1.1.tar.gz"),
        ];

        let variants = extract_tool_variants(&downloads);

        assert_eq!(variants.len(), 2);
        // Known (Native) sorts before the opaque default entry.
        assert_eq!(
            variants[0].choice,
            ToolVariantChoice::Known(ToolVariant::Native)
        );
        assert_eq!(
            variants[1].choice,
            ToolVariantChoice::Opaque("dxvk-3.1.1.tar.gz".into())
        );
    }

    #[test]
    fn test_extract_tool_variants_sorts_microarch_order() {
        let downloads = vec![
            create_mock_download("proton-x86_64_v3.tar.gz"),
            create_mock_download("proton-x86_64.tar.gz"),
            create_mock_download("proton-x86_64_v4.tar.gz"),
            create_mock_download("proton-x86_64_v2.tar.gz"),
        ];

        let variants = extract_tool_variants(&downloads);

        let known: Vec<ToolVariant> = variants
            .iter()
            .filter_map(|v| match v.choice {
                ToolVariantChoice::Known(inner) => Some(inner),
                _ => None,
            })
            .collect();
        assert_eq!(
            known,
            vec![
                ToolVariant::X86_64,
                ToolVariant::X86_64V2,
                ToolVariant::X86_64V3,
                ToolVariant::X86_64V4,
            ]
        );
    }

    #[test]
    fn test_extract_tool_variants_keeps_unknown_as_is() {
        let downloads = vec![
            create_mock_download("some-other-file.tar.gz"),
            create_mock_download("another-file.tar.gz"),
        ];

        let variants = extract_tool_variants(&downloads);

        assert_eq!(variants.len(), 2);
        assert!(
            variants
                .iter()
                .all(|v| matches!(v.choice, ToolVariantChoice::Opaque(_)))
        );
        // Opaque entries preserve asset order.
        assert_eq!(variants[0].file_name(), "some-other-file.tar.gz");
        assert_eq!(variants[1].file_name(), "another-file.tar.gz");
    }

    #[test]
    fn test_extract_tool_variants_dedupes_by_file_name() {
        let downloads = vec![
            create_mock_download("proton-x86_64.tar.gz"),
            create_mock_download("proton-x86_64.tar.gz"),
        ];

        let variants = extract_tool_variants(&downloads);

        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn test_details_display_includes_file_name() {
        let details = ToolVariantDetails {
            choice: ToolVariantChoice::Known(ToolVariant::Wow64),
            download: create_mock_download("proton-wineland-11.0-x86_64_wow64.tar.xz"),
        };
        assert_eq!(
            details.to_string(),
            "WoW64 - 64-bit only (proton-wineland-11.0-x86_64_wow64.tar.xz)"
        );
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
    fn test_select_default_variant_handles_opaque_first() {
        let downloads = vec![create_mock_download("dxvk-3.1.1.tar.gz")];

        let default = select_default_variant(&downloads).unwrap();
        assert_eq!(default.file_name, "dxvk-3.1.1.tar.gz");
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
