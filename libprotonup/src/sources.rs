use super::apps;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

const SOURCES_RON: &str = include_str!("sources.ron");

lazy_static! {
    pub static ref CompatTools: Vec<CompatTool> = ron::from_str(SOURCES_RON).unwrap();
}

/// Struct used to build GitHub API request URLs.
///
/// Contains the GitHub URL, the username for GE,
/// the repository name for either Wine GE or Proton GE,
/// and a Variant Enum for identifying the parameters type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompatTool {
    /// compat_tool name
    pub name: String,
    /// the forge from witch this program will get the tool
    pub forge: Forge,
    /// GitHub account for the variant
    pub repository_account: String,
    /// name of the repository
    pub repository_name: String,
    /// compatible with these applications
    pub compatible_applications: Vec<apps::App>,
    /// ToolType can be used to change how it is installed
    pub tool_type: ToolType,

    /// release asset filter drops unwanted release assets (checksums, source
    /// archives, unrelated build artefacts). It matches on the *stable* part of
    /// the asset name only; the version segment is treated as opaque so that
    /// tools are free to renumber their releases without breaking installs.
    /// Archive extension validation is NOT part of the filter; it is handled by
    /// `files::check_supported_extension`.
    pub release_asset_filter: Option<AssetFilter>,

    // Templates in order:
    /// file_name_replacement does a replace_all to the text version
    pub file_name_replacement: Option<(String, String)>,
    /// file_name_template will add prefixes and suffixes to
    /// the final installation folder.
    /// The template must contain "{version}"
    /// it is applied after the replacement, if Some()
    pub file_name_template: Option<String>,

    /// if true, a menu to choose the variant to download will be shown (for proton cachyos: x86_64, x86_64_v2, x86_64_v3, x86_64_v4)
    #[serde(default)]
    pub has_multiple_asset_variations: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Forges are from where the tools should be downloaded
/// new forges should be implemented when a tool is hosted
/// in a not yet supported forge
pub enum Forge {
    GitHub,
    Custom(String),
}

impl Forge {
    // A method to get the static string slice for the enum variant
    pub fn get_url(&self) -> &str {
        match self {
            Forge::GitHub => "https://api.github.com/repos",
            Forge::Custom(url) => url,
        }
    }

    // A method to get the user-facing url
    pub fn get_user_url(&self) -> &str {
        match self {
            Forge::GitHub => "https://github.com/",
            Forge::Custom(url) => url,
        }
    }
}

/// Matches release asset names on their stable prefix (and optionally suffix).
///
/// The version portion of an asset name is deliberately not described here: any
/// upstream renumbering (e.g. `3.1.1` -> `3.0b` -> `4-rc1`) still matches.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum AssetFilter {
    /// Asset name must start with this literal.
    Prefix(String),
    /// Asset name must start with any of these literals. Used by tools that
    /// changed their naming scheme over time (e.g. `GE-Proton` vs `Proton-`).
    Prefixes(Vec<String>),
    /// Asset name must start with the prefix AND end with the suffix.
    /// The suffix must not include the archive extension, which is validated
    /// separately by `files::check_supported_extension`.
    PrefixSuffix(String, String),
}

impl AssetFilter {
    /// Returns true when `path` satisfies the filter.
    pub fn matches(&self, path: &str) -> bool {
        match self {
            AssetFilter::Prefix(prefix) => path.starts_with(prefix.as_str()),
            AssetFilter::Prefixes(prefixes) => {
                prefixes.iter().any(|p| path.starts_with(p.as_str()))
            }
            AssetFilter::PrefixSuffix(prefix, suffix) => {
                path.starts_with(prefix.as_str()) && path.ends_with(suffix.as_str())
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// ToolTypes
pub enum ToolType {
    WineBased,
    Runtime,
}

impl CompatTool {
    /// new_custom is a generator for custom VariantParameters
    #[allow(clippy::too_many_arguments)]
    pub fn new_custom(
        name: String,
        forge: Forge,
        repository_account: String,
        repository_name: String,
        tool_type: ToolType,
        release_asset_filter: Option<AssetFilter>,
        file_name_replacement: Option<(String, String)>,
        file_name_template: Option<String>,
    ) -> CompatTool {
        CompatTool {
            name,
            forge,
            repository_account,
            repository_name,
            tool_type,
            release_asset_filter,
            file_name_replacement,
            file_name_template,
            has_multiple_asset_variations: false,
            compatible_applications: vec![], // TODO: fill this if it becomes helpful
        }
    }

    // installation_dir applies file_name filters defined for each compat_tool,
    // and returns the final installation directory
    pub fn installation_name(&self, version: &str) -> String {
        let mut name = match &self.file_name_replacement {
            Some(replacement) => version.replace(&replacement.0, &replacement.1).to_owned(),
            None => version.to_owned(),
        };
        name = match &self.file_name_template {
            Some(template) => template.replace("{version}", name.as_str()),
            None => name,
        };
        name
    }

    pub fn sources_for_app(app: &apps::App) -> Vec<CompatTool> {
        CompatTools
            .iter()
            .cloned()
            .to_owned()
            .filter(move |s| s.compatible_applications.contains(app))
            .collect()
    }

    /// filter_asset checks the file name against the tool's [`AssetFilter`].
    /// Returns true when no filter is defined.
    ///
    /// The filter only inspects the stable name prefix/suffix; see
    /// [`AssetFilter`]. Archive extensions are validated elsewhere.
    pub fn filter_asset(&self, path: &str) -> bool {
        match &self.release_asset_filter {
            Some(asset_filter) => asset_filter.matches(path),
            None => true,
        }
    }

    /// filter_asset_with_arch is like [`filter_asset`], but also matches assets
    /// that carry a known architecture suffix (e.g. `GE-Proton11-4-x86_64.tar.gz`).
    /// The asset matches if either its original name or its name with the
    /// architecture suffix stripped satisfies [`filter_asset`].
    ///
    /// Returns a tuple of the matching asset and file name (if any).
    pub fn filter_asset_with_arch(
        &self,
        path: &str,
    ) -> (bool, Option<super::architecture::CpuArch>) {
        let (stripped, arch) = super::architecture::strip_arch_suffix(path);
        let matched = self.filter_asset(path) || (stripped != path && self.filter_asset(&stripped));
        (matched, arch)
    }
}

impl fmt::Display for CompatTool {
    /// Returns a string representation of this Variant
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn normalize_tool_name(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '(' | ')'))
        .map(|c| if c == '_' { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl FromStr for CompatTool {
    type Err = ();
    fn from_str(input: &str) -> Result<CompatTool, Self::Err> {
        let normalized = normalize_tool_name(input);
        for s in CompatTools.iter() {
            if normalize_tool_name(&s.name) == normalized {
                return Ok(s.clone());
            }
        }
        Err(())
    }
}

#[cfg(test)] // Only compile this module when running `cargo test`
mod tests {
    // Import the functions from the parent module (or wherever they are defined)
    use super::*;

    #[test]
    fn test_from_str_underscore_and_case_insensitive() {
        assert_eq!(
            CompatTool::from_str("Proton_CachyOS").unwrap().name,
            "Proton CachyOS"
        );
        assert_eq!(
            CompatTool::from_str("kron4ek_wine").unwrap().name,
            "Kron4ek Wine"
        );
        assert_eq!(CompatTool::from_str("geproton").unwrap().name, "GEProton");
        assert_eq!(
            CompatTool::from_str("GEProton_RTSP").unwrap().name,
            "GEProton RTSP"
        );
        assert_eq!(
            CompatTool::from_str("WineGE_Deprecated").unwrap().name,
            "WineGE (Deprecated)"
        );
        assert_eq!(
            CompatTool::from_str("WineGE_(Deprecated)").unwrap().name,
            "WineGE (Deprecated)"
        );
        assert!(CompatTool::from_str("does-not-exist").is_err());
    }

    // ---------------------------------------------------------------------
    // Asset filter test corpus
    //
    // Asset names are frozen from live GitHub releases (collected 2026-11) so
    // that regressions against real-world naming are caught. Each case is a
    // (tool, asset_name, expected_filter_match) triple evaluated through the
    // *real* `sources.ron` definitions via `CompatTool::from_str`.
    //
    // Note: this table exercises `filter_asset` only. Extension validation is
    // the responsibility of `files::check_supported_extension` and is covered
    // in `files::tests`.
    // ---------------------------------------------------------------------
    const ASSET_FILTER_CASES: &[(&str, &str, bool)] = &[
        // --- GEProton: two historical prefix eras ---
        ("GEProton", "GE-Proton11-12-x86_64.tar.gz", true),
        ("GEProton", "GE-Proton11-12-aarch64.tar.gz", true),
        ("GEProton", "GE-Proton9-24.tar.gz", true),
        ("GEProton", "Proton-6.1-GE-2.tar.gz", true), // older `Proton-` era
        ("GEProton", "GE-Proton-1.2.3.4.tar.gz", true), // arbitrary tail: opaque version
        ("GEProton", "GE-Proton11-12-x86_64.sha512sum", true), // prefix matches; extension filters it in downloads.rs
        // --- DXVK: trailing dash is load bearing ---
        ("DXVK", "dxvk-3.1.1.tar.gz", true),
        ("DXVK", "dxvk-3.1.tar.gz", true),
        ("DXVK", "dxvk-1.tar.gz", true), // renumbering still matches
        ("DXVK", "dxvk-2.6.1.beta.tar.gz", true), // pre-release tail
        ("DXVK", "dxvk-native-3.1.1-steamrt-sniper.tar.gz", true),
        ("DXVK", "dxvk-gplasync-3.1.1.tar.gz", true),
        ("DXVK", "notdxvk-1.0.tar.gz", false),
        // --- Kron4ek Wine ---
        ("Kron4ek Wine", "wine-11.21-amd64.tar.xz", true),
        ("Kron4ek Wine", "wine-11.21-amd64-wow64.tar.xz", true),
        ("Kron4ek Wine", "wine-11.21-staging-amd64.tar.xz", true),
        ("Kron4ek Wine", "wine-11.21-staging-tkg-x86.tar.xz", true),
        ("Kron4ek Wine", "sha256sums.txt", false),
        // --- GEProton RTSP: old and new eras ---
        ("GEProton RTSP", "GE-Proton10-33-rtsp24-1.tar.gz", true),
        ("GEProton RTSP", "proton-rtsp-11.0-20260609-4.tar.gz", true), // new era
        (
            "GEProton RTSP",
            "proton-rtsp-11.0-20260609-4.tar.gz.sha512sum",
            true,
        ), // prefix matches; extension filters it
        // --- Proton CachyOS ---
        (
            "Proton CachyOS",
            "proton-cachyos-11.0-20261015-slr-x86_64.tar.xz",
            true,
        ),
        (
            "Proton CachyOS",
            "proton-cachyos-11.0-20261015-slr-x86_64_v3.tar.xz",
            true,
        ),
        (
            "Proton CachyOS",
            "proton-cachyos-11.0-20261015-slr-arm64.tar.xz",
            true,
        ),
        (
            "Proton CachyOS",
            "proton-cachyos-11.0-20261015-slr-x86_64_v3.sha512sum",
            true,
        ), // prefix matches; excluded by extension in downloads.rs
        // --- Proton Wineland (incl. non-arch `wow64` token) ---
        (
            "Proton Wineland",
            "proton-wineland-11.0-20261106-x86_64.tar.xz",
            true,
        ),
        (
            "Proton Wineland",
            "proton-wineland-11.0-20261106-x86_64_v3.tar.xz",
            true,
        ),
        (
            "Proton Wineland",
            "proton-wineland-11.0-20261106-x86_64_wow64.tar.xz",
            true,
        ),
        (
            "Proton Wineland",
            "proton-wineland-11.0-20261106-x86_64_v3.sha512sum",
            true,
        ), // prefix matches; excluded by extension in downloads.rs
        // --- Tools without a filter: everything passes `filter_asset` ---
        ("Luxtorpeda", "luxtorpeda-v76.3.0.tar.xz", true),
        ("Boxtron", "boxtron.tar.xz", true),
        ("VKD3D-Proton", "vkd3d-proton-3.0b.tar.zst", true), // letter-suffix version
        ("Lutris-VKD3D", "vkd3d-2.14.tar.xz", true),
        (
            "WineGE (Deprecated)",
            "wine-lutris-GE-Proton8-26-x86_64.tar.xz",
            true,
        ),
        (
            "WineGE (Deprecated)",
            "wine-lutris-GE-Proton8-27-LoL-x86_64.tar.xz",
            true,
        ),
    ];

    #[test]
    fn test_filter_asset_real_world_corpus() {
        for (tool, asset, expected) in ASSET_FILTER_CASES {
            // Every tool referenced here must exist in sources.ron.
            let ct = CompatTool::from_str(tool)
                .unwrap_or_else(|_| panic!("tool '{tool}' not found in sources.ron"));
            let actual = ct.filter_asset(asset);
            assert_eq!(
                actual, *expected,
                "filter_asset('{asset}') for tool '{tool}': expected {expected}, got {actual}"
            );
        }
    }

    /// `filter_asset_with_arch` must compose the arch-suffix strip with the
    /// prefix filter, and must not strip non-arch tokens such as `wow64`.
    #[test]
    fn test_filter_asset_with_arch_composition() {
        let cases: &[(&str, &str, bool, Option<crate::architecture::CpuArch>)] = &[
            (
                "Proton CachyOS",
                "proton-cachyos-11.0-20261015-slr-x86_64_v3.tar.xz",
                true,
                Some(crate::architecture::CpuArch::X86),
            ),
            (
                "Proton CachyOS",
                "proton-cachyos-11.0-20261015-slr-arm64.tar.xz",
                true,
                Some(crate::architecture::CpuArch::Arm),
            ),
            (
                "Proton Wineland",
                "proton-wineland-11.0-20261106-x86_64_wow64.tar.xz",
                true,
                Some(crate::architecture::CpuArch::X86),
            ),
            (
                "GEProton",
                "GE-Proton16-6-x86.tar.gz",
                true,
                Some(crate::architecture::CpuArch::X86),
            ),
            (
                "GEProton",
                "GE-Proton26-9-aarch64_v8.tar.gz",
                true,
                Some(crate::architecture::CpuArch::Arm),
            ),
            ("Luxtorpeda", "luxtorpeda-v76.3.0.tar.xz", true, None),
        ];

        for (tool, asset, expected, expected_arch) in cases {
            let ct = CompatTool::from_str(tool).unwrap();
            let (matched, arch) = ct.filter_asset_with_arch(asset);
            assert_eq!(
                matched, *expected,
                "filter_asset_with_arch('{asset}') for '{tool}': expected {expected}, got {matched}"
            );
            assert_eq!(
                arch, *expected_arch,
                "arch detection for '{asset}' ('{tool}')"
            );
        }
    }

    /// The `AssetFilter` variants behave as documented.
    #[test]
    fn test_asset_filter_matches() {
        let prefix = AssetFilter::Prefix("dxvk-".into());
        assert!(prefix.matches("dxvk-3.1.1.tar.gz"));
        assert!(!prefix.matches("notdxvk-3.1.1.tar.gz"));
        assert!(!prefix.matches(""));
        assert!(prefix.matches("dxvk-native-3.1.1.tar.gz"));
        assert!(prefix.matches("dxvk-3.1.1.tar.gz"));
        assert!(prefix.matches("dxvk-3.1.tar.gz"));
        assert!(prefix.matches("dxvk-1.tar.gz"));
        assert!(prefix.matches("dxvk-2.6.1.beta.tar.gz"));
        assert!(prefix.matches("dxvk-native-3.1.1.tar.gz"));
        assert!(prefix.matches("dxvk-gplasync-3.1.1.tar.gz"));
        assert!(prefix.matches("dxvk-.tar.gz"));
        assert!(prefix.matches("dxvk-"));
        assert!(!prefix.matches("notdxvk-3.1.1.tar.gz"));

        let prefixes = AssetFilter::Prefixes(vec!["GE-Proton".into(), "Proton-".into()]);
        assert!(prefixes.matches("GE-Proton11-12.tar.gz"));
        assert!(prefixes.matches("Proton-6.1-GE-2.tar.gz"));
        assert!(!prefixes.matches("proton-rtsp-11.0.tar.gz"));

        let both = AssetFilter::PrefixSuffix("wine-".into(), "-amd64.tar.xz".into());
        assert!(both.matches("wine-11.21-amd64.tar.xz"));
        assert!(!both.matches("wine-11.21-x86.tar.xz"));
        assert!(!both.matches("wine-11.21-amd64.tar.xz.bak"));

        // No filter -> everything matches.
        let ct = CompatTool::from_str("Luxtorpeda").unwrap();
        assert!(ct.filter_asset("anything-at-all.tgz"));
        assert!(ct.filter_asset(""));
    }

    /// Every tool in sources.ron must parse with the new `AssetFilter` type.
    #[test]
    fn test_all_tools_parse() {
        assert!(!CompatTools.is_empty(), "sources.ron yielded no tools");
        for tool in CompatTools.iter() {
            assert!(!tool.name.is_empty(), "tool with empty name in sources.ron");
        }
    }
}
