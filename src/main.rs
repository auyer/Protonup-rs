use structopt::StructOpt;

use tokio;

#[derive(Debug, StructOpt)]
struct Opt {
    /// install a specific version
    #[structopt(short, long)]
    tag: Option<String>,
    #[structopt(short, long)]
    /// list installed versions
    list: Option<String>,
    /// remove existing installations
    #[structopt(short, long)]
    remove: Option<String>,
    /// set specific output
    #[structopt(short, long)]
    output: Option<String>,
    /// set installation directory
    #[structopt(short, long)]
    dir: Option<String>,
    /// disable prompts and logs
    #[structopt(short, long)]
    yes: bool,
    /// download only
    #[structopt(long)]
    download: bool,
    /// list available versions
    #[structopt(long)]
    releases: bool,
}
#[tokio::main]
async fn main() {
    let Opt {
        tag,
        list,
        remove,
        output,
        dir,
        yes,
        download,
        releases,
    } = Opt::from_args();

    if releases {
        //|| !tag.is_none() {
        println!("releases");
        let mut release_list = protonup_rs::list_releases().await.unwrap();
        if releases {
            release_list.into_iter().map(|r| println!("{}", r.tag_name));
            return;
        }
        // if !tag.is_none() {
        // 	let tag = tag.unwrap();
        // 	let tag_list: Vec<octocrab::models::repos::Release> =
        // 		release_list.drain(..).filter(|r| &r.tag_name == &tag).collect();
        // 	if tag_list.len() > 1 {}
        // }
    }

    if !tag.is_none() {
        protonup_rs::download_file(&tag.unwrap()).await;
    } else {
        protonup_rs::download_file("latest").await;
    }
    // version=None, yes=True, dl_only=False, output=None

    // Ok(())
    // ()
}
