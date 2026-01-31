use std::path::PathBuf;

use super::constants;
use crate::apps;
use crate::files;
use crate::hashing;
use crate::sources::CompatTool;
use anyhow::{self, Context, Result};
use futures_util::TryStreamExt;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use tokio::io::{self, AsyncWrite};
use tokio_util::io::StreamReader;

pub type ReleaseList = Vec<Release>;

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
    /// For most tools, where there is no variation in a single release
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
            } else if compat_tool.filter_asset(asset.download_file_name().as_str())
                && files::check_supported_extension(&asset.name).is_ok()
            {
                download.file_name = asset.name.clone();
                download
                    .download_url
                    .clone_from(&asset.browser_download_url);
                download.size = asset.size as u64;
                break;
            }
        }
        download
    }

    /// Returns all Download structs corresponding to the Release for tools with multiple asset variations
    /// This is used for tools like ProtonCachyOS that offer builds optimized for different CPU microarchitectures
    pub fn get_all_download_variants(
        &self,
        for_app: &apps::AppInstallations,
        compat_tool: &CompatTool,
    ) -> Vec<Download> {
        // Create a map from base filename to hash file URL
        let mut asset_hashsum_map: std::collections::HashMap<
            String,
            (String, hashing::HashSumType),
        > = std::collections::HashMap::new();

        // Build the hash map by matching hash files to their base names
        for asset in &self.assets {
            if asset.name.contains("sha512") {
                // hash_type = hashing::HashSumType::Sha512;
                let base_name = asset.name.split(".sha512").collect::<Vec<&str>>()[0].to_owned();
                asset_hashsum_map.insert(
                    base_name,
                    (
                        asset.browser_download_url.clone(),
                        hashing::HashSumType::Sha512,
                    ),
                );
            } else if asset.name.contains("sha256") {
                // hash_type = hashing::HashSumType::Sha256;
                let base_name = asset.name.split(".sha256").collect::<Vec<&str>>()[0].to_owned();
                asset_hashsum_map.insert(
                    base_name,
                    (
                        asset.browser_download_url.clone(),
                        hashing::HashSumType::Sha256,
                    ),
                );
            }
        }

        let mut variants = Vec::new();

        // Collect all matching asset variants with their corresponding hash
        for asset in &self.assets {
            if compat_tool.filter_asset(asset.download_file_name().as_str())
                && files::check_supported_extension(&asset.name).is_ok()
            {
                let base_name = asset
                    .name
                    .strip_suffix(".tar.gz")
                    .or_else(|| asset.name.strip_suffix(".tar.xz"))
                    .or_else(|| asset.name.strip_suffix(".tar.zst"))
                    .unwrap_or(&asset.name);

                let hash_sum = asset_hashsum_map
                    .get(base_name)
                    .map(|(hash_url, hash_type)| hashing::HashSums {
                        sum_content: hash_url.clone(),
                        sum_type: hash_type.clone(),
                    });

                variants.push(Download {
                    file_name: asset.name.clone(),
                    download_url: asset.browser_download_url.clone(),
                    size: asset.size as u64,
                    for_app: for_app.to_owned(),
                    version: self.tag_name.clone(),
                    hash_sum,
                });
            }
        }

        variants
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
    pub fn download_file_name(&self) -> String {
        self.browser_download_url
            .split('/')
            .next_back()
            .unwrap_or(&self.browser_download_url)
            .to_owned()
    }
}

/// Downloads and returns the text response
pub async fn download_file_into_memory(url: &String) -> Result<String> {
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, format!("protonup-rs {}", constants::VERSION))
        .send()
        .await
        .with_context(|| {
            format!(
                "[Download SHA] Failed to call remote server on URL : {}",
                &url
            )
        })?;

    res.text()
        .await
        .with_context(|| format!("[Download SHA] Failed to read response from URL : {}", &url))
}

/// Downloads to a AsyncWrite buffer, where hooks and Wrappers can be used to report progress
pub async fn download_to_async_write<W: AsyncWrite + Unpin>(
    url: &str,
    write: &mut W,
) -> Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, format!("protonup-rs {}", constants::VERSION))
        .send()
        .await
        .with_context(|| format!("[Download] Failed to call remote server on URL : {}", &url))?;

    io::copy(
        &mut StreamReader::new(res.bytes_stream().map_err(io::Error::other)),
        write,
    )
    .await?;
    Ok(())
}

/// Returns a Vec of Releases from a GitHub repository, the URL used for the request is built from the passed in VariantParameters
pub async fn list_releases(compat_tool: &CompatTool) -> Result<ReleaseList, reqwest::Error> {
    let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

    let url = format!(
        "{}/{}/{}/releases",
        compat_tool.forge.get_url(),
        compat_tool.repository_account,
        compat_tool.repository_name,
    );

    let client = reqwest::Client::builder().user_agent(agent).build()?;

    let r_list: ReleaseList = client.get(url).send().await?.json().await?;

    // filter releases without assets
    let r_list: ReleaseList = r_list
        .into_iter()
        .filter(|rel| {
            !rel.assets.is_empty()
                // same logic used when creating the Release object
                && (rel.assets.iter().any(|asset| {
                    compat_tool.filter_asset(asset.download_file_name().as_str())
                        && files::check_supported_extension(&asset.name).is_ok()
                }))
        })
        .collect();

    Ok(r_list)
}

/// Contains all the information needed to download the corresponding release from GitHub
#[derive(Default, Debug, PartialEq, Clone)]
pub struct Download {
    /// file name should be used to verify checksums if available
    pub file_name: String,
    /// for what app this download is
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
            .keep();

        match files::check_supported_extension(&self.download_url) {
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
                "case : '{desc}' test: list_releases returned error"
            );

            let result = result.unwrap();

            assert!(
                result.len() > 1,
                "case : '{desc}' test: test_list_releases returned an empty list"
            );
        }
    }

    #[tokio::test]
    async fn test_get_release() {
        let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

        let client = match reqwest::Client::builder().user_agent(agent).build() {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Error: {e}");
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
                source_parameters.forge.get_url(),
                source_parameters.repository_account,
                source_parameters.repository_name
            );

            let rel = match client.get(url).send().await {
                Ok(res) => res,
                Err(e) => {
                    panic!("Error: {e}");
                }
            }
            .json::<Release>()
            .await;

            assert!(rel.is_ok(), "case : '{desc}' test: test_get_release wrong");
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
            println!("Input: {input:#?}");
            println!("Output: {output:?}");
            println!("Expected: {expected:?}");
            assert!(output == expected, "{output} Should match: {expected}");
        }
    }

    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_list_releases_mocked() {
        let mock_server = MockServer::start().await;

        let owner = "test-owner";
        let repo_valid = "test-repo-with-valid-asset";
        let expected_path_valid = format!("/{}/{}/releases", owner, repo_valid);

        // mock data
        let mock_response_valid = json!([
            {
                "tag_name": "GE-Proton9-10-rtsp12",
                "name": "GE-Proton9-10-rtsp12",
                "assets": [
                    {
                        "url": "https://api.github.com/asset1",
                        "id": 1,
                        "name": "GE-Proton9-10-rtsp13.tar.gz",
                        "size": 1024,
                        "updated_at": "2024-01-01T00:00:00Z",
                        "browser_download_url": format!("{}/releases/download/GE-Proton9-10-rtsp13/GE-Proton9-10-rtsp13.tar.gz", mock_server.uri())
                    },
                    {
                        "url": "https://api.github.com/asset1",
                        "id": 1,
                        "name": "Source Code",
                        "size": 1024,
                        "updated_at": "2024-01-01T00:00:00Z",
                        "browser_download_url": format!("{}/releases/download/some-other-asset-hotfix.tar.gz", mock_server.uri())
                    }
                ]
            }
        ]);

        // Setup the mock behavior
        Mock::given(method("GET"))
            .and(path(expected_path_valid)) // Ensure this matches the tool below
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response_valid))
            .expect(1)
            .mount(&mock_server)
            .await;

        let repo_no_asset = "test-repo-no-asset";
        let expected_path_invalid = format!("/{}/{}/releases", owner, repo_no_asset);

        // mock data
        let mock_response_invalid = json!([
            {
                "tag_name": "GE-Proton9-10-rtsp12",
                "name": "GE-Proton9-10-rtsp12",
                "assets": [
                    {
                        "url": "https://api.github.com/asset1",
                        "id": 1,
                        "name": "Source Code",
                        "size": 1024,
                        "updated_at": "2024-01-01T00:00:00Z",
                        "browser_download_url": format!("{}/releases/download/some-other-asset-hotfix.tar.gz", mock_server.uri())
                    }
                ]
            }
        ]);

        Mock::given(method("GET"))
            .and(path(expected_path_invalid)) // Ensure this matches the tool below
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response_invalid))
            .expect(1)
            .mount(&mock_server)
            .await;

        let regex =
            r"^(GE-Proton|Proton-)[0-9]+(-[0-9]+)?-(rtsp)-?(\d+(-\d+)?)?\.(tar\.gz|tar\.zst)$";

        // TEST CASE ONE: tool has an asset with expected filename
        let tool = CompatTool::new_custom(
            "TestTool".to_string(),
            sources::Forge::Custom(mock_server.uri()), // Injection point
            owner.to_string(),
            repo_valid.to_string(),
            sources::ToolType::WineBased,
            Some(regex.to_owned()),
            None,
            None,
        );

        // Call the function pointing to the mock server
        let result = list_releases(&tool).await.expect("Request failed");

        assert_eq!(result.len(), 1, "Should have assets");
        assert_eq!(result[0].tag_name, "GE-Proton9-10-rtsp12");

        // TEST CASE TWO: tool has no valid asset
        let tool = CompatTool::new_custom(
            "TestTool".to_string(),
            sources::Forge::Custom(mock_server.uri()), // Injection point
            owner.to_string(),
            repo_no_asset.to_string(),
            sources::ToolType::WineBased,
            None,
            None,
            None,
        );

        // Call the function pointing to the mock server
        let result = list_releases(&tool).await.expect("Request failed");

        assert_eq!(
            result.len(),
            0,
            "Should have filtered out the release without assets"
        );
    }
}
