use super::apps;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

const SOURCCES_RON: &str = include_str!("sources.ron");

lazy_static! {
    pub static ref Sources: Vec<Source> = ron::from_str(SOURCCES_RON).unwrap();
}

/// Struct used to build GitHub API request URLs.
///
/// Contains the GitHub URL, the username for GE,
/// the repository name for either Wine GE or Proton GE,
/// and a Variant Enum for identifying the parameters type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Source {
    /// source name
    pub name: String,
    /// the forge from witch this program will get the tool
    pub forge: Forge,
    /// GitHub account for the variant
    pub repository_account: String,
    /// name of the repository
    pub repository_name: String,
    /// compatible with these applications
    pub compatible_applications: Vec<apps::App>,
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

impl Source {
    /// new_custom is a generator for custom VariantParameters
    pub fn new_custom(
        name: String,
        forge: Forge,
        repository_account: String,
        repository_name: String,
        file_name_replacement: Option<(String, String)>,
        file_name_template: Option<String>,
    ) -> Source {
        Source {
            name,
            forge,
            repository_account,
            repository_name,
            file_name_replacement,
            file_name_template,
            compatible_applications: vec![], // TODO: fill this if it becomes helpful
        }
    }

    pub fn sources_for_app(app: apps::App) -> Vec<Source> {
        Sources
            .iter()
            .cloned()
            .to_owned()
            .filter(move |s| s.compatible_applications.contains(&app))
            .collect()
    }
}

impl fmt::Display for Source {
    /// Returns a string representation of this Variant
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl FromStr for Source {
    type Err = ();
    fn from_str(input: &str) -> Result<Source, Self::Err> {
        for s in Sources.iter() {
            if s.name.to_lowercase() == input.to_lowercase() {
                return Ok(s.clone());
            }
        }
        Err(())
    }
}
