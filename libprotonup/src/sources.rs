use super::apps;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

const SOURCCES_RON: &str = include_str!("sources.ron");

lazy_static! {
    pub static ref CompatTools: Vec<CompatTool> = ron::from_str(SOURCCES_RON).unwrap();
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

    // Templates in order :
    /// file_name_replacement does a replace_all to the text version
    pub file_name_replacement: Option<(String, String)>,
    /// file_name_template will add prefixes and suffixes to
    /// the final installation folder.
    /// The template must contain "{version}"
    /// it is applied after the replacement, if Some()
    pub file_name_template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Forges are from where the tools should be downloaded
/// new forges should be implemented when a tool is hosted
/// in a not yet supported forge
pub enum Forge {
    GitHub,
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
                "Regex test failed for input: '{}'. Expected {}, got {}",
                input, expected, actual
            );
        }
    }
}
