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

    /// release asset filter is a regex to filter out uwanted release assets
    pub release_asset_filter: Option<String>,

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
        release_asset_filter: Option<String>,
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

    /// filter_asset executes a regex on the file name to determine if the asset found matches
    /// returns true if No filter defined, and false if the filter does not compile
    pub fn filter_asset(&self, path: &str) -> bool {
        match self.release_asset_filter.clone() {
            Some(asset_filter) => match regex::Regex::new(&asset_filter) {
                Ok(re) => re.is_match(path),
                Err(_) => false,
            },
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

impl FromStr for CompatTool {
    type Err = ();
    fn from_str(input: &str) -> Result<CompatTool, Self::Err> {
        for s in CompatTools.iter() {
            if s.name.to_lowercase() == input.to_lowercase() {
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

    const TEST_CASES: &[(&str, bool)] = &[
        // --- Valid Cases ---
        ("dxvk-2.6.1.tar.gz", true),    // Standard 3-part version
        ("dxvk-2.6.tar.gz", true),      // Standard 2-part version
        ("dxvk-10.20.30.tar.gz", true), // Multi-digit 3-part version
        ("dxvk-10.20.tar.gz", true),    // Multi-digit 2-part version
        ("dxvk-0.0.0.tar.gz", true),    // Zeroes are valid digits
        ("dxvk-0.1.tar.gz", true),      // Zeroes are valid digits
        // --- Invalid Cases ---
        ("dxvk-invalid.zip", false),       // Wrong suffix
        ("dxvk-2.tar.gz", false), // Requires Major.Minor minimum (fails regex \d+\.\d+ and manual split len check)
        ("dxvk-2.6.1.beta.tar.gz", false), // Extra text in version part
        ("dxvk-.tar.gz", false),  // Missing version part
        ("dxvk-2.6..tar.gz", false), // Double dot in version part
        ("prefix-dxvk-2.6.tar.gz", false), // Incorrect prefix
        ("dxvk-2.6.tar.gz-suffix", false), // Incorrect suffix
        ("", false),              // Empty string
        ("dxvk-a.b.tar.gz", false), // Non-digits in version part
        ("dxvk-1.2.3.4.tar.gz", false), // Too many version parts
        ("dxvk-1.2 .tar.gz", false), // Space in version part
        ("dxvk-1..tar.gz", false), // Double dot variant
        ("dxvk-1.2.", false),     // Wrong suffix / incomplete
        (".tar.gz", false),       // Missing prefix and version
        ("dxvk-", false),         // Missing version and suffix
    ];

    #[test]
    fn test_is_dxvk_archive_name_regex_table() {
        let empty = "".to_owned();
        // example regex for dxvk
        let dxvk_regex = r"^dxvk-\d+\.\d+(?:\.\d+)?\.tar\.gz$";

        for (input, expected) in TEST_CASES {
            let s = CompatTool::new_custom(
                empty.clone(),
                Forge::GitHub,
                empty.clone(),
                empty.clone(),
                ToolType::Runtime,
                Some(dxvk_regex.to_owned()),
                None,
                None,
            );
            let actual = s.filter_asset(input.to_owned());
            assert_eq!(
                actual, *expected,
                "Regex test failed for input: '{input}'. Expected {expected}, got {actual}"
            );
        }
    }

    const TEST_CASES_PROTONGE: &[(&str, bool)] = &[
        // --- Valid Cases ---
        ("GE-Proton10-8.tar.gz", true),
        ("GE-Proton10-8.tar.zst", true),
        ("GE-Proton9-24.tar.gz", true),
        ("GE-Proton-2.tar.gz", false), // not a real version, but should be flexible
        ("Proton-4.20-GE-1.tar.gz", true),
        ("Proton-6.1-GE-2.tar.gz", true),
        // --- Invalid Cases ---
        ("", false),                         // Empty string
        ("GE-Proton-1.2.3.4.tar.gz", false), // Too many version parts
        ("GE-Proton-1.2 .tar.gz", false),    // Space in version part
        ("GE-Proton-1..tar.gz", false),      // Double dot variant
        ("GE-Proton-1.2.", false),           // Wrong suffix / incomplete
        (".tar.gz", false),                  // Missing prefix and version
        ("GE-Proton-", false),               // Missing version and suffix
        ("GE-Proton9-24.sha512sum", false),  // checksum file
    ];

    #[test]
    fn test_is_protonge_archive_name_regex_table() {
        let empty = "".to_owned();
        // example regex for GE-Proton
        let regex =
            r"^(GE-Proton|Proton-)[0-9]+(-[0-9]+)?(\.\d+(\.\d+)?)?(-GE-\d+)?\.(tar\.gz|tar\.zst)$";

        for (input, expected) in TEST_CASES_PROTONGE {
            let s = CompatTool::new_custom(
                empty.clone(),
                Forge::GitHub,
                empty.clone(),
                empty.clone(),
                ToolType::Runtime,
                Some(regex.to_owned()),
                None,
                None,
            );
            let actual = s.filter_asset(input.to_owned());
            assert_eq!(
                actual, *expected,
                "Regex test failed for input: '{input}'. Expected {expected}, got {actual}"
            );
        }
    }

    /// Multi-architecture asset matching: each entry is (tool, file name, matches, detected arch).
    const TEST_CASES_MULTI_ARCH: &[(&str, &str, bool, Option<crate::architecture::CpuArch>)] = &[
        // GE-Proton 11-4: both architectures carry an explicit suffix
        (
            "GEProton",
            "GE-Proton11-4-x86_64.tar.gz",
            true,
            Some(crate::architecture::CpuArch::X86),
        ),
        (
            "GEProton",
            "GE-Proton11-4-aarch64.tar.gz",
            true,
            Some(crate::architecture::CpuArch::Arm),
        ),
        (
            "GEProton",
            "GE-Proton11-5-aarch64.tar.gz",
            true,
            Some(crate::architecture::CpuArch::Arm),
        ),
        (
            "GEProton",
            "GE-Proton11-5-x86_64.tar.gz",
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
            "GE-Proton26-9-x86_64_v7.tar.gz", // can you imagine?
            true,
            Some(crate::architecture::CpuArch::X86),
        ),
        (
            "GEProton",
            "GE-Proton26-9-aarch64_v8.tar.gz", // can you imagine?
            true,
            Some(crate::architecture::CpuArch::Arm),
        ),
        // GE-Proton 11-3: default (no suffix) x86 + aarch64
        ("GEProton", "GE-Proton11-3.tar.gz", true, None),
        (
            "GEProton",
            "GE-Proton11-3-aarch64.tar.gz",
            true,
            Some(crate::architecture::CpuArch::Arm),
        ),
        // GE-Proton 11-2: x86 only, no suffix
        ("GEProton", "GE-Proton11-2.tar.gz", true, None),
        // Other projects
        (
            "Kron4ek Wine",
            "wine-9.0-amd64.tar.xz",
            true,
            Some(crate::architecture::CpuArch::X86),
        ),
        (
            "Proton CachyOS",
            "proton-cachyos-9.0-20250101-abc123-x86_64_v3.tar.gz",
            true,
            Some(crate::architecture::CpuArch::X86),
        ),
        (
            "Proton CachyOS",
            "proton-cachyos-11.0-20260703-slr-x86_64_v3.tar.xz",
            true,
            Some(crate::architecture::CpuArch::X86),
        ),
        (
            "Proton CachyOS",
            "proton-cachyos-11.0-20260703-slr-x86_64.tar.xz",
            true,
            Some(crate::architecture::CpuArch::X86),
        ),
        (
            "Proton CachyOS",
            "proton-cachyos-11.0-20260703-slr-arm64.tar.xz",
            true,
            Some(crate::architecture::CpuArch::Arm),
        ),
        ("Luxtorpeda", "Luxtorpeda-v76.2.0.tar.xz", true, None),
        ("GEProton RTSP", "GE-Proton9-10-rtsp12.tar.gz", true, None),
        ("DXVK", "dxvk-2.6.1.tar.gz", true, None),
    ];

    #[test]
    fn test_is_multi_arch_archive_name_regex_table() {
        let empty = "".to_owned();

        let regex_for = |tool: &str| -> Option<String> {
            match tool {
                "GEProton" => Some(
                    r"^(GE-Proton|Proton-)[0-9]+(-[0-9]+)?(\.\d+(\.\d+)?)?(-GE-\d+)?\.(tar\.gz|tar\.zst)$".to_owned(),
                ),
                "Kron4ek Wine" => {
                    Some(r"^wine-\d+\.\d+(?:\.\d+)?\.tar\.xz$".to_owned())
                }
                "Proton CachyOS" => Some(
                    r"^proton-cachyos-[0-9]+\.[0-9]+-[0-9]+-[a-z0-9]+\.(tar\.gz|tar\.xz|tar\.zst)$".to_owned(),
                ),
                "Luxtorpeda" => None,
                "GEProton RTSP" => Some(
                    r"^(GE-Proton|Proton-)[0-9]+(-[0-9]+)?-(rtsp)-?(\d+(-\d+)?)?\.(tar\.gz|tar\.zst)$".to_owned(),
                ),
                "DXVK" => Some(r"^dxvk-\d+\.\d+(?:\.\d+)?\.tar\.gz$".to_owned()),
                _ => None,
            }
        };

        for (tool, input, expected, expected_arch) in TEST_CASES_MULTI_ARCH {
            let s = CompatTool::new_custom(
                empty.clone(),
                Forge::GitHub,
                empty.clone(),
                empty.clone(),
                ToolType::Runtime,
                regex_for(tool),
                None,
                None,
            );
            let (actual, actual_arch) = s.filter_asset_with_arch(input);
            assert_eq!(
                actual, *expected,
                "Regex test failed for input: '{input}'. Expected {expected}, got {actual}"
            );
            assert_eq!(
                actual_arch, *expected_arch,
                "Arch detection failed for input: '{input}'. Expected {expected_arch:?}, got {actual_arch:?}"
            );
        }
    }
}
