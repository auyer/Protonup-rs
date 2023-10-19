use clap::Parser;

use inquire::Select;

use std::fmt;

use libprotonup::apps::App;

mod download;
mod file_path;
mod helper_menus;
mod manage_apps;

use manage_apps::manage_apps_routine;

#[derive(Debug, Parser)]
struct Opt {
    /// Skip Menu, auto detect apps and download using default parameters
    #[arg(short, long)]
    quick_download: bool,
}

#[derive(Debug, Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
enum InitialMenu {
    QuickUpdate,
    DownloadForSteam,
    DownloadForLutris,
    DownloadIntoCustomLocation,
    ManageExistingInstallations,
}

impl InitialMenu {
    // could be generated by macro
    const VARIANTS: &'static [InitialMenu] = &[
        Self::QuickUpdate,
        Self::DownloadForSteam,
        Self::DownloadForLutris,
        Self::DownloadIntoCustomLocation,
        Self::ManageExistingInstallations,
    ];
}

impl fmt::Display for InitialMenu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::QuickUpdate => write!(f, "Quick Update (detect apps and auto download)"),
            Self::DownloadForSteam => write!(f, "Download GE-Proton for Steam"),
            Self::DownloadForLutris => write!(f, "Download GE-Proton/Wine-GE for Lutris"),
            Self::DownloadIntoCustomLocation => {
                write!(f, "Download GE-Proton/Wine-GE into custom location")
            }
            Self::ManageExistingInstallations => write!(f, "Manage Existing Proton Installations"),
        }
    }
}

#[tokio::main]
async fn main() {
    // run quick downloads and skip InitialMenu
    let Opt { quick_download } = Opt::parse();
    if quick_download {
        download::run_quick_downloads().await
    } else {
        let answer: InitialMenu = Select::new(
            "ProtonUp Menu: Chose your action:",
            InitialMenu::VARIANTS.to_vec(),
        )
        .with_page_size(10)
        .prompt()
        .unwrap_or_else(|_| std::process::exit(0));

        // Set parameters based on users choice
        match answer {
            InitialMenu::QuickUpdate => download::run_quick_downloads().await,
            InitialMenu::DownloadForSteam => {
                download::download_to_selected_app(Some(App::Steam)).await
            }

            //     selected_app = Some(apps::App::Steam);
            //     should_open_tag_selector = true;
            // }
            InitialMenu::DownloadForLutris => {
                download::download_to_selected_app(Some(App::Lutris)).await
            }
            //     selected_app = Some(apps::App::Lutris);
            //     should_open_tag_selector = true;
            // }
            InitialMenu::DownloadIntoCustomLocation => {
                download::download_to_selected_app(None).await
            }
            //     should_open_dir_selector = true;
            //     should_open_tag_selector = true;
            // }
            InitialMenu::ManageExistingInstallations => manage_apps_routine(),
        }
    }
}
