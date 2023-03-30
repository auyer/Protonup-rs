use iced_native::subscription;

use libprotonup::{constants, file, github, utils};
use std::fs;
use std::fs::create_dir_all;
use std::hash::Hash;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Just a little utility function
pub fn file<I: 'static + Hash + Copy + Send + Sync, T: ToString>(
    id: I,
    url: T,
) -> iced::Subscription<(I, Progress)> {
    subscription::unfold(id, State::Ready(url.to_string()), move |state| {
        download(id, state)
    })
}

async fn download<I: Copy>(id: I, state: State) -> (Option<(I, Progress)>, State) {
    let install_path = constants::DEFAULT_INSTALL_DIR;
    let tag = github::fetch_data_from_tag("latest", false).await.unwrap();

    if file::check_if_exists(
        constants::DEFAULT_INSTALL_DIR.to_owned(),
        tag.version.clone(),
    ) {
        // popup confirm overwrite
        // if !confirm_menu(format!(
        //     "Version {} exists in installation path. Overwrite ?",
        //     tag.version
        // )) {
        //     return;
        // }
    }

    let install_dir = utils::expand_tilde(install_path).unwrap();
    let mut temp_dir = utils::expand_tilde(constants::TEMP_DIR).unwrap();

    let download = github::fetch_data_from_tag(&tag.version, false)
        .await
        .unwrap();

    temp_dir.push(format!("{}.tar.gz", &download.version));

    // install_dir
    create_dir_all(&install_dir).unwrap();

    let git_hash = file::download_file_into_memory(&download.sha512sum)
        .await
        .unwrap();

    if temp_dir.exists() {
        fs::remove_file(&temp_dir).unwrap();
    }
    println!(" Setting up download");
    let (progress, done) = file::create_progress_trackers();
    let progress_read = Arc::clone(&progress);
    let done_read = Arc::clone(&done);
    let url = String::from(&download.download);
    let i_dir = String::from(install_dir.to_str().unwrap());

    // thread::spawn(move || {
    match state {
        State::Ready(url) => {
            println!(" starting up download");

            file::download_file_progress(
                download.download,
                download.size,
                &temp_dir,
                progress,
                done,
            )
            .await
            .unwrap();
            // let response = reqwest::get(&url).await;

            // match response {
            //     Ok(response) => {
            //         if let Some(total) = response.content_length() {
            //             (
            //                 Some((id, Progress::Started)),
            //                 State::Downloading {
            //                     response,
            //                     total,
            //                     downloaded: 0,
            //                 },
            //             )
            //         } else {
            //             (Some((id, Progress::Errored)), State::Finished)
            //         }
            //     }
            //     Err(_) => (Some((id, Progress::Errored)), State::Finished),
            //
            // }
            //
            return (
                Some((id, Progress::Started)),
                State::Downloading {
                    // response,
                    total: download.size,
                    downloaded: 0,
                },
            );
        }
        State::Downloading {
            // mut response,
            total,
            downloaded,
        } => {
            println!(" ckecking up download");
            let newpos = progress_read.load(Ordering::Relaxed);

            // /                match response.chunk().await {
            // Ok(Some(chunk)) => {
            let downloaded = newpos as u64;

            let percentage = (downloaded as f32 / total as f32) * 100.0;

            return (
                Some((id, Progress::Advanced(percentage))),
                State::Downloading {
                    // response,
                    total,
                    downloaded,
                },
            );
            // }
            // Ok(None) => (Some((id, Progress::Finished)), State::Finished),
            // Err(_) => (Some((id, Progress::Errored)), State::Finished),
        }
        State::Finished => {
            // We do not let the stream die, as it would start a
            // new download repeatedly if the user is not careful
            // in case of errors.
            // iced::futures::future::pending().await
            return (Some((id, Progress::Finished)), State::Finished);
        }
    }
    // });
}

#[derive(Debug, Clone)]
pub enum Progress {
    Started,
    Advanced(f32),
    Finished,
    Errored,
}

pub enum State {
    Ready(String),
    Downloading {
        // response: reqwest::Response,
        total: u64,
        downloaded: u64,
    },
    Finished,
}
