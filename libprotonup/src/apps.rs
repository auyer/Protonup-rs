use arcstr::ArcStr;
use futures_util::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fmt, str::FromStr};

use crate::sources::ToolType;
use crate::{
    constants,
    files::{self, list_folders_in_path},
    sources::CompatTool,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
pub enum App {
    Steam,
    Lutris,
    // TODO:  HeroicGamesLauncher,
}

/// APP_VARIANTS is a shorthand to all app variants
pub static APP_VARIANTS: &[App] = &[App::Steam, App::Lutris];

impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Steam => write!(f, "Steam"),
            Self::Lutris => write!(f, "Lutris"),
        }
    }
}

impl App {
    /// Returns the default compatibility tool for the App
    pub fn default_compatibility_tool(&self) -> CompatTool {
        match *self {
            // TODO: this could fail if the default apps change
            Self::Steam => CompatTool::from_str(constants::DEFAULT_STEAM_TOOL).unwrap(),
            Self::Lutris => CompatTool::from_str(constants::DEFAULT_LUTRIS_TOOL).unwrap(),
        }
    }

    /// Returns the variantst of AppInstallations corresponding to the App
    pub fn app_installations(&self) -> Vec<AppInstallations> {
        match *self {
            Self::Steam => vec![AppInstallations::Steam, AppInstallations::SteamFlatpak],
            Self::Lutris => vec![AppInstallations::Lutris, AppInstallations::LutrisFlatpak],
        }
    }

    /// Checks the versions (Native vs Flatpak) of the App that are installed
    pub async fn detect_installation_method(&self) -> Vec<AppInstallations> {
        match *self {
            Self::Steam => {
                detect_installations(&[AppInstallations::Steam, AppInstallations::SteamFlatpak])
                    .await
            }
            Self::Lutris => {
                detect_installations(&[AppInstallations::Lutris, AppInstallations::LutrisFlatpak])
                    .await
            }
        }
    }

    pub fn list_subfolders(&self) -> Option<Vec<&str>> {
        match self {
            App::Steam => None,
            App::Lutris => Some(vec!["runners/wine", "runtime"]),
        }
    }

    // returns the subfolder from the App base path if the app and tool requires it,
    // or an empty string if not required
    pub fn subfolder_for_tool(&self, source: &CompatTool) -> &str {
        match self {
            App::Steam => "",
            App::Lutris => match source.tool_type {
                ToolType::WineBased => "runners/wine",
                ToolType::Runtime => "runtime",
            },
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum AppInstallations {
    #[default]
    Steam,
    SteamFlatpak,
    Lutris,
    LutrisFlatpak,
}

impl fmt::Display for AppInstallations {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Steam => write!(f, "Steam \"Native\" "),
            Self::SteamFlatpak => write!(f, "Steam Flatpak"),
            Self::Lutris => write!(f, "Lutris \"Native\""),
            Self::LutrisFlatpak => write!(f, "Lutris Flatpak"),
        }
    }
}

impl AppInstallations {
    /// Default directory that wine is extracted to
    pub const fn default_install_dir(&self) -> ArcStr {
        match *self {
            Self::Steam => arcstr::literal!("~/.steam/steam/compatibilitytools.d/"),
            Self::SteamFlatpak => {
                arcstr::literal!(
                    "~/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d/"
                )
            }
            Self::Lutris => arcstr::literal!("~/.local/share/lutris/"),
            Self::LutrisFlatpak => {
                arcstr::literal!("~/.var/app/net.lutris.Lutris/data/lutris/")
            }
        }
    }

    /// The app root folder
    pub const fn app_base_dir(&self) -> &'static str {
        match *self {
            Self::Steam => "~/.steam/steam/",
            Self::SteamFlatpak => "~/.var/app/com.valvesoftware.Steam/data/Steam/",
            Self::Lutris => "~/.local/share/lutris/",
            Self::LutrisFlatpak => "~/.var/app/net.lutris.Lutris/data/lutris/",
        }
    }

    /// Get a list of the currently installed wine versions
    pub async fn list_installed_versions(&self) -> Result<Vec<String>, anyhow::Error> {
        let base_dir = self.default_install_dir().to_string();
        match self.as_app().list_subfolders() {
            Some(sub_folders) => {
                let mut versions = Vec::new();
                for sub_folder in sub_folders {
                    let path = PathBuf::from(&base_dir).join(sub_folder);
                    let folders = list_folders_in_path(&path).await?;
                    versions.extend(folders);
                }
                Ok(versions)
            }
            None => list_folders_in_path(&PathBuf::from(&base_dir)).await,
        }
    }

    /// Returns the base App
    pub const fn as_app(&self) -> App {
        match *self {
            Self::Steam | Self::SteamFlatpak => App::Steam,
            Self::Lutris | Self::LutrisFlatpak => App::Lutris,
        }
    }
}

/// list_installed_apps returns a vector of App variants that are installed
pub async fn list_installed_apps() -> Vec<AppInstallations> {
    detect_installations(APP_INSTALLATIONS_VARIANTS).await
}

/// detect_installations returns a vector of App variants that are detected
async fn detect_installations(app_installations: &[AppInstallations]) -> Vec<AppInstallations> {
    stream::iter(app_installations)
        .filter_map(|app| async move {
            if files::check_if_exists(app.app_base_dir(), ".").await {
                Some(app.clone())
            } else {
                None
            }
        })
        .map(|app| app.clone())
        .collect()
        .await
}

/// APP_INSTALLATIONS_VARIANTS contains the subset of variants of the App enum that are actual apps
pub static APP_INSTALLATIONS_VARIANTS: &[AppInstallations] = &[
    AppInstallations::Steam,
    AppInstallations::SteamFlatpak,
    AppInstallations::Lutris,
    AppInstallations::LutrisFlatpak,
];
