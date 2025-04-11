use std::path::PathBuf;

use crate::apps;
use crate::constants;
use crate::files;
use crate::hashing;
use crate::sources::CompatTool;
use anyhow::Result;
use serde::{Deserialize, Serialize};
pub type ReleaseList = Vec<Release>;

pub const GITHUB_URL: &str = "https://api.github.com/repos";

/// Contains the information from one of the releases on GitHub
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Release {
    /// API URL of the Release
    url: Option<String>,
    /// Tag name of the Release, examples "8.7-GE-1-Lol" "GE-Proton8-5"
    pub tag_name: String,
    name: String,
    /// Asset list for each Release, usually the tar.gz/tar.xz file and a sha512sum file for integrity checking
    assets: Vec<Asset>,
}

impl std::fmt::Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.tag_name)
    }
}

impl Release {
    /// Returns a Download struct corresponding to the Release
    pub fn get_download_info(
        &self,
        for_app: &apps::AppInstallations,
        compat_tool: &CompatTool,
    ) -> Download {
        let mut download: Download = Download {
            for_app: for_app.to_owned(),
            version: self.tag_name.clone(),
            ..Download::default()
        };
        for asset in &self.assets {
            if asset.name.contains("sha512") {
                download.file_name = asset.name.clone();
                download.hash_sum = Some(hashing::HashSums {
                    sum_content: asset.browser_download_url.clone(),
                    sum_type: hashing::HashSumType::Sha512,
                })
            } else if asset.name.contains("sha256") {
                download.file_name = asset.name.clone();
                download.hash_sum = Some(hashing::HashSums {
                    sum_content: asset.browser_download_url.clone(),
                    sum_type: hashing::HashSumType::Sha256,
                })
            } else if compat_tool.filter_asset(asset.dowload_file_name().as_str())
                && files::check_supported_extension(asset.name.clone()).is_ok()
            {
                download.file_name = asset.name.clone();
                download
                    .download_url
                    .clone_from(&asset.browser_download_url);
                download.size = asset.size as u64;
            }
        }
        download
    }
}

/// Holds the information from the different Assets for each GitHub release
///
/// An Asset could be for the wine tar folder or for the sha512sum
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Asset {
    url: String,
    id: i64,
    name: String,
    size: i64,
    updated_at: String,
    browser_download_url: String,
}

impl Asset {
    pub fn dowload_file_name(&self) -> String {
        self.browser_download_url
            .split('/')
            .next_back()
            .unwrap_or(&self.browser_download_url)
            .to_owned()
    }
}

/// Returns a Vec of Releases from a GitHub repository, the URL used for the request is built from the passed in VariantParameters
pub async fn list_releases(compat_tool: &CompatTool) -> Result<ReleaseList, reqwest::Error> {
    let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

    let url = format!(
        "{}/{}/{}/releases",
        GITHUB_URL, compat_tool.repository_account, compat_tool.repository_name,
    );

    let client = reqwest::Client::builder().user_agent(agent).build()?;

    let r_list: ReleaseList = client.get(url).send().await?.json().await?;

    Ok(r_list)
}

/// Contains all the information needed to download the corresponding release from GitHub
#[derive(Default, Debug, PartialEq, Clone)]
pub struct Download {
    /// file name should be used to verify checksums if available
    pub file_name: String,
    /// for what app this dowload is
    pub for_app: apps::AppInstallations,
    /// the tag from the Forge
    pub version: String,
    /// file hash to check download integrity
    pub hash_sum: Option<hashing::HashSums>,
    /// Download URL for the release's compressed tar file
    pub download_url: String,
    /// The reported size of the tar download
    pub size: u64,
}

impl Download {
    // output_dir checks if the file is supported and returns the standardized file name
    pub fn download_dir(&self) -> Result<PathBuf> {
        let mut output_dir = tempfile::tempdir()
            .expect("Failed to create tempdir")
            .into_path();

        match files::check_supported_extension(self.download_url.clone()) {
            Ok(ext) => {
                output_dir.push(format!("{}.{}", &self.version, ext));
                Ok(output_dir)
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{constants, sources};

    use super::*;

    #[tokio::test]
    async fn test_list_releases() {
        let conditions = &[
            (
                sources::CompatTool::from_str(constants::DEFAULT_LUTRIS_TOOL).unwrap(),
                "Get WineGE",
            ),
            (
                sources::CompatTool::from_str(constants::DEFAULT_STEAM_TOOL).unwrap(),
                "Get GEProton",
            ),
            (
                sources::CompatTool::from_str("Luxtorpeda").unwrap(),
                "Get Luxtorpeda",
            ),
        ];

        for (source_parameters, desc) in conditions {
            let result = list_releases(source_parameters).await;

            assert!(
                result.is_ok(),
                "case : '{}' test: list_releases returned error",
                desc
            );

            let result = result.unwrap();

            assert!(
                result.len() > 1,
                "case : '{}' test: test_list_releases returned an empty list",
                desc
            );
        }
    }

    #[tokio::test]
    async fn test_get_release() {
        let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

        let client = match reqwest::Client::builder().user_agent(agent).build() {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1)
            }
        };

        let conditions = &[
            (
                sources::CompatTool::from_str(constants::DEFAULT_LUTRIS_TOOL).unwrap(),
                "Get WineGE",
            ),
            (
                sources::CompatTool::from_str(constants::DEFAULT_STEAM_TOOL).unwrap(),
                "Get GEProton",
            ),
        ];
        for (source_parameters, desc) in conditions {
            let url = format!(
                "{}/{}/{}/releases/latest",
                GITHUB_URL, source_parameters.repository_account, source_parameters.repository_name
            );

            let rel = match client.get(url).send().await {
                Ok(res) => res,
                Err(e) => {
                    panic!("Error: {}", e);
                }
            }
            .json::<Release>()
            .await;

            assert!(
                rel.is_ok(),
                "case : '{}' test: test_get_release wrong",
                desc
            );
        }
    }

    #[tokio::test]
    async fn test_get_download_name() {
        let empty = "".to_owned();

        let test_cases = vec![
            // "GE-Proton
            (
                Download {
                    file_name: "GE-Proton9-27.tar.gz".to_owned(),
                    version: "GE-Proton9-27".to_owned(),
                    hash_sum: None,
                    for_app: apps::AppInstallations::Steam,
                    size: 0,
                    download_url: empty.clone(),
                },
                CompatTool::from_str("GEProton").unwrap(),
                "GE-Proton9-27",
            ),
            // WineGE
            (
                Download {
                    file_name: "wine-lutris-GE-Proton8-26-x86_64.tar.xz".to_owned(),
                    version: "GE-Proton8-26".to_owned(),
                    for_app: apps::AppInstallations::Steam,
                    hash_sum: None,
                    size: 0,
                    download_url: empty.clone(),
                },
                CompatTool::from_str("WineGE").unwrap(),
                "GE-Wine8-26",
            ),
        ];

        for (input, compat_tool, expected) in test_cases {
            let output = compat_tool.installation_name(&input.version);
            println!("Input: {:#?}", input);
            println!("Output: {:?}", output);
            println!("Expected: {:?}", expected);
            assert!(output == expected, "{} Should match: {}", output, expected);
        }
    }
}
