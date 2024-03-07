use crate::{
    files::{self, list_folders_in_path},
    variants::Variant,
};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    /// Returns the version of Wine used for the App
    pub fn app_wine_version(&self) -> Variant {
        match *self {
            Self::Steam => Variant::GEProton,
            Self::Lutris => Variant::WineGE,
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
    pub fn detect_installation_method(&self) -> Vec<AppInstallations> {
        match *self {
            Self::Steam => {
                detect_installations(&[AppInstallations::Steam, AppInstallations::SteamFlatpak])
            }
            Self::Lutris => {
                detect_installations(&[AppInstallations::Lutris, AppInstallations::LutrisFlatpak])
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AppInstallations {
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
    pub fn default_install_dir(&self) -> &'static str {
        match *self {
            Self::Steam => "~/.steam/steam/compatibilitytools.d/",
            Self::SteamFlatpak => {
                "~/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d/"
            }
            Self::Lutris => "~/.local/share/lutris/runners/wine/",
            Self::LutrisFlatpak => "~/.var/app/net.lutris.Lutris/data/lutris/runners/wine/",
        }
    }

    /// The app root folder
    pub fn app_base_dir(&self) -> &'static str {
        match *self {
            Self::Steam => "~/.steam/steam/",
            Self::SteamFlatpak => "~/.var/app/com.valvesoftware.Steam/data/Steam/",
            Self::Lutris => "~/.local/share/lutris/",
            Self::LutrisFlatpak => "~/.var/app/net.lutris.Lutris/data/lutris/",
        }
    }

    /// Get a list of the currently installed wine versions
    pub fn list_installed_versions(&self) -> Result<Vec<String>, anyhow::Error> {
        list_folders_in_path(self.default_install_dir())
    }

    /// Returns the base App
    pub fn into_app(&self) -> App {
        match *self {
            Self::Steam | Self::SteamFlatpak => App::Steam,
            Self::Lutris | Self::LutrisFlatpak => App::Lutris,
        }
    }
}

/// list_installed_apps returns a vector of App variants that are installed
pub fn list_installed_apps() -> Vec<AppInstallations> {
    detect_installations(APP_INSTALLATIONS_VARIANTS)
}

/// detect_installations returns a vector of App variants that are detected
fn detect_installations(app_installations: &[AppInstallations]) -> Vec<AppInstallations> {
    app_installations
        .iter()
        .filter(|app| files::check_if_exists(app.app_base_dir(), ""))
        .cloned()
        .collect()
}

/// APP_INSTALLATIONS_VARIANTS contains the subset of variants of the App enum that are actual apps
pub static APP_INSTALLATIONS_VARIANTS: &[AppInstallations] = &[
    AppInstallations::Steam,
    AppInstallations::SteamFlatpak,
    AppInstallations::Lutris,
    AppInstallations::LutrisFlatpak,
];
