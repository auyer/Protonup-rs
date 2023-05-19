use crate::constants;
use crate::parameters::VariantParameters;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub type ReleaseList = Vec<Release>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Release {
    /// API URL of the Release
    url: String,
    /// Tag name of the Release, examples "8.7-GE-1-Lol" "GE-Proton8-5"
    pub tag_name: String,
    /// Release post name, examples "Wine-GE-Proton8-5 Released" " Lutris-GE-8.7-1-LoL"
    name: String,
    published_at: String,
    /// Asset list for each Release, usually the tar.gz/tar.xz file and a sha512sum file for integrity checking
    assets: Vec<Asset>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Asset {
    /// API URL of the Asset
    url: String,
    /// API ID of the Asset
    id: i64,
    /// File name of the Asset
    name: String,
    /// Size in Bytes, divide by 1_000_000 for MB
    size: i64,
    created_at: String,
    updated_at: String,
    /// Direct download URL
    browser_download_url: String,
}

/// Returns a Vec of Releases from a GitHub repository, the URL used for the request is built from the passed in VariantParameters
pub async fn list_releases(source: &VariantParameters) -> Result<ReleaseList, reqwest::Error> {
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
    /// Proton or Wine GE version, based off tag
    pub version: String,
    /// URL to download the sha512sum for this Download
    pub sha512sum_url: String,
    /// URL to download the Wine or Proton archive
    pub download_url: String,
    /// Size of Wine or Proton archive in Bytes
    pub size: u64,
    pub created_at: String,
}

/// Returns a Download struct from the passed in VariantParameters and version tag.
pub async fn fetch_data_from_tag(
    tag: &str,
    source: &VariantParameters,
) -> Result<Download, reqwest::Error> {
    let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

    let client = reqwest::Client::builder().user_agent(agent).build()?;

    let mut download = Download::default();
    let url = format!(
        "{}/{}/{}/releases/{}",
        source.repository_url, source.repository_account, source.repository_name, tag
    );
    let release: Release = client.get(url).send().await?.json().await?;

    download.version = release.tag_name;
    for asset in &release.assets {
        if asset.name.ends_with("sha512sum") {
            download.sha512sum_url = asset.browser_download_url.as_str().to_string();
        }
        if asset.name.ends_with("tar.gz") || asset.name.ends_with("tar.xz") {
            download.created_at = asset.created_at.clone();
            download.download_url = asset.browser_download_url.as_str().to_string();
            download.size = asset.size as u64;
        }
    }
    Ok(download)
}

#[cfg(test)]
mod tests {
    use crate::parameters;

    use super::*;

    #[tokio::test]
    async fn test_fetch_data_from_tag() {
        let conditions = &[
            (
                parameters::Variant::WineGE.parameters(),
                "latest",
                "Get Steam",
            ),
            (
                parameters::Variant::GEProton.parameters(),
                "latest",
                "Download Lutris",
            ),
        ];
        for (source_parameters, tag, desc) in conditions {
            let result = fetch_data_from_tag(tag, source_parameters).await;

            assert!(
                result.is_ok(),
                "case :{} test: fetch_data_from_tag returned error",
                desc
            );

            let result = result.unwrap();

            assert!(
                result.download_url.len() > 5,
                "case : '{}' test: fetch_data_from_tag returned an wrong download link",
                desc
            );
            assert!(
                result.sha512sum_url.len() > 5,
                "case : '{}' test: fetch_data_from_tag returned an wrong sha512sum",
                desc
            );
            assert!(
                result.size > 100,
                "case : '{}' test: fetch_data_from_tag returned an wrong sha512sum",
                desc
            );
            assert!(
                result.version.len() > 2,
                "case : '{}' test: fetch_data_from_tag returned an wrong version",
                desc
            );
        }
    }

    #[tokio::test]
    async fn test_list_releases() {
        let conditions = &[
            (parameters::Variant::WineGE.parameters(), "List WineGE"),
            (parameters::Variant::GEProton.parameters(), "List GEProton"),
        ];

        for (source_parameters, desc) in conditions {
            let result = list_releases(source_parameters).await;

            assert!(
                result.is_ok(),
                "case : '{}' test: fetch_data_from_tag returned error",
                desc
            );

            let result = result.unwrap();

            println!("Got result: {result:?}");

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
            (parameters::Variant::WineGE.parameters(), "Get WineGE"),
            (parameters::Variant::GEProton.parameters(), "Get GEProton"),
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

            println!("Got result: {rel:?}");

            assert!(
                rel.is_ok(),
                "case : '{}' test: test_get_release wrong",
                desc
            );
        }
    }
}
