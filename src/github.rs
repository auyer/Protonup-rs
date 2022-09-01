const GITHUB_REPO: &str = "proton-ge-custom";
const GITHUB_ACCOUNT: &str = "GloriousEggroll";

#[derive(Default, Debug, PartialEq)]
pub struct Download {
    pub version: String,
    pub date: String,
    pub sha512sum: String,
    pub download: String,
    pub size: u64,
}

pub async fn list_releases() -> Result<Vec<octocrab::models::repos::Release>, octocrab::Error> {
    let releases = octocrab::instance()
        .repos(GITHUB_ACCOUNT, GITHUB_REPO)
        .releases()
        .list()
        .per_page(10)
        .page(1u32)
        .send()
        // .get_latest()
        .await?
        .take_items();
    Ok(releases)
}

pub async fn fetch_data(tag: &str) -> Result<Download, octocrab::Error> {
    let mut download = Download::default();
    let release = match tag {
        "latest" => {
            octocrab::instance()
                .repos(GITHUB_ACCOUNT, GITHUB_REPO)
                .releases()
                .get_latest()
                .await?
        }
        _ => {
            octocrab::instance()
                .repos(GITHUB_ACCOUNT, GITHUB_REPO)
                .releases()
                .get_by_tag(tag)
                .await?
        }
    };

    download.version = release.tag_name;
    // download.date = release.published_at;
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
