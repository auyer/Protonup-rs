use super::constants;

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Download {
    pub version: String,
    pub sha512sum: String,
    pub download: String,
    pub size: u64,
}

pub async fn list_releases() -> Result<Vec<octocrab::models::repos::Release>, octocrab::Error> {
    let releases = octocrab::instance()
        .repos(constants::GITHUB_ACCOUNT, constants::GITHUB_REPO)
        .releases()
        .list()
        .per_page(30)
        .page(1u32)
        .send()
        .await?
        .take_items();
    Ok(releases)
}

pub async fn fetch_data_from_tag(tag: &str) -> Result<Download, octocrab::Error> {
    let mut download = Download::default();
    let release = match tag {
        "latest" => {
            octocrab::instance()
                .repos(constants::GITHUB_ACCOUNT, constants::GITHUB_REPO)
                .releases()
                .get_latest()
                .await?
        }
        _ => {
            octocrab::instance()
                .repos(constants::GITHUB_ACCOUNT, constants::GITHUB_REPO)
                .releases()
                .get_by_tag(tag)
                .await?
        }
    };

    download.version = release.tag_name;
    for ass in &release.assets {
        if ass.name.ends_with("sha512sum") {
            download.sha512sum = ass.browser_download_url.as_str().to_string();
        }
        if ass.name.ends_with("tar.gz") {
            download.download = ass.browser_download_url.as_str().to_string();
            download.size = ass.size as u64;
        }
    }
    Ok(download)
}
