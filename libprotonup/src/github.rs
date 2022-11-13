use super::constants;
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

pub async fn list_releases() -> Result<ReleaseList, reqwest::Error> {
    let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

    let url = format!(
        "{}/{}/{}/releases",
        constants::GITHUB,
        constants::GITHUB_ACCOUNT,
        constants::GITHUB_REPO,
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

pub async fn fetch_data_from_tag(tag: &str) -> Result<Download, reqwest::Error> {
    let agent = format!("{}/v{}", constants::USER_AGENT, constants::VERSION,);

    let client = reqwest::Client::builder().user_agent(agent).build()?;

    let mut download = Download::default();
    let release = match tag {
        "latest" => {
            let url = format!(
                "{}/{}/{}/releases/latest",
                constants::GITHUB,
                constants::GITHUB_ACCOUNT,
                constants::GITHUB_REPO,
            );
            let rel: Release = client.get(url).send().await?.json().await?;
            rel
        }
        _ => {
            let url = format!(
                "{}/{}/{}/releases/tags/{}",
                constants::GITHUB,
                constants::GITHUB_ACCOUNT,
                constants::GITHUB_REPO,
                &tag
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
    }
    Ok(download)
}
