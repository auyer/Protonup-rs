use crate::constants;
use crate::variants::VariantGithubParameters;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub type ReleaseList = Vec<Release>;

#[derive(Serialize, Deserialize, Debug)]
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
    pub fn get_download_info(&self) -> Download {
        let mut download: Download = Download::default();
        download.version = self.tag_name.clone();
        for asset in &self.assets {
            if asset.name.ends_with("sha512sum") {
                download.sha512sum_url = asset.browser_download_url.clone();
            } else if asset.name.ends_with("tar.gz") || asset.name.ends_with("tar.xz") {
                download.download_url = asset.browser_download_url.clone();
                download.size = asset.size as u64;
            }
        }

        download
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Asset {
    url: String,
    id: i64,
    name: String,
    size: i64,
    updated_at: String,
    browser_download_url: String,
}

/// Returns a Vec of Releases from a GitHub repository, the URL used for the request is built from the passed in VariantParameters
pub async fn list_releases(
    source: &VariantGithubParameters,
) -> Result<ReleaseList, reqwest::Error> {
    let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

    let url = format!(
        "{}/{}/{}/releases",
        source.repository_url, source.repository_account, source.repository_name,
    );

    let client = reqwest::Client::builder().user_agent(agent).build()?;

    let r_list: ReleaseList = client.get(url).send().await?.json().await?;

    Ok(r_list)
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Download {
    pub version: String,
    pub sha512sum_url: String,
    pub download_url: String,
    pub size: u64,
}
#[cfg(test)]
mod tests {
    use crate::variants;

    use super::*;

    #[tokio::test]
    async fn test_list_releases() {
        let conditions = &[
            (
                variants::Variant::WineGE.get_github_parameters(),
                "List WineGE",
            ),
            (
                variants::Variant::GEProton.get_github_parameters(),
                "List GEProton",
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
                variants::Variant::WineGE.get_github_parameters(),
                "Get WineGE",
            ),
            (
                variants::Variant::GEProton.get_github_parameters(),
                "Get GEProton",
            ),
        ];
        for (source_parameters, desc) in conditions {
            let url = format!(
                "{}/{}/{}/releases/latest",
                source_parameters.repository_url,
                source_parameters.repository_account,
                source_parameters.repository_name
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
}
