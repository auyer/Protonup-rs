use crate::constants;
use crate::parameters::VariantParameters;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub type ReleaseList = Vec<Release>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Release {
    url: String,
    pub tag_name: String,
    name: String,
    published_at: String,
    assets: Vec<Asset>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Asset {
    url: String,
    id: i64,
    name: String,
    size: i64,
    created_at: String,
    updated_at: String,
    browser_download_url: String,
}

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
    pub version: String,
    pub sha512sum: String,
    pub download: String,
    pub size: u64,
    pub created_at: String,
}

pub async fn fetch_data_from_tag(
    tag: &str,
    source: &VariantParameters,
) -> Result<Download, reqwest::Error> {
    let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

    let client = reqwest::Client::builder().user_agent(agent).build()?;

    let mut download = Download::default();
    let release = match tag {
        "latest" => {
            let url = format!(
                "{}/{}/{}/releases/latest",
                source.repository_url, source.repository_account, source.repository_name,
            );
            let rel: Release = client.get(url).send().await?.json().await?;
            rel
        }
        _ => {
            let url = format!(
                "{}/{}/{}/releases/tags/{}",
                source.repository_url, source.repository_account, source.repository_name, &tag
            );
            let rel: Release = client.get(url).send().await?.json().await?;
            rel
        }
    };

    download.version = release.tag_name;
    for asset in &release.assets {
        if asset.name.ends_with("sha512sum") {
            download.sha512sum = asset.browser_download_url.as_str().to_string();
        }
        if asset.name.ends_with("tar.gz") {
            download.created_at = asset.created_at.clone();
            download.download = asset.browser_download_url.as_str().to_string();
            download.size = asset.size as u64;
        }
        if asset.name.ends_with("tar.xz") {
            download.created_at = asset.created_at.clone();
            download.download = asset.browser_download_url.as_str().to_string();
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
                result.download.len() > 5,
                "case : '{}' test: fetch_data_from_tag returned an wrong download link",
                desc
            );
            assert!(
                result.sha512sum.len() > 5,
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

            assert!(
                rel.is_ok(),
                "case : '{}' test: test_get_release wrong",
                desc
            );
        }
    }
}
