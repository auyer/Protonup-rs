use crate::files::list_folders_in_path;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum App {
    Steam,
    SteamFlatpak,
    Lutris,
    LutrisFlatpak,
    DetectAll,
}

impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Steam => write!(f, "Steam"),
            Self::SteamFlatpak => write!(f, "Steam Flatpak"),
            Self::Lutris => write!(f, "Lutris"),
            Self::LutrisFlatpak => write!(f, "Lutris Flatpak"),
            Self::DetectAll => write!(f, "Detect All"),
        }
    }
}

impl App {
    pub fn default_install_dir(&self) -> &'static str {
        match *self {
            Self::Steam => "~/.steam/steam/compatibilitytools.d/",
            Self::SteamFlatpak => {
                "~/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d/"
            }
            Self::Lutris => "~/.local/share/lutris/runners/wine/",
            Self::LutrisFlatpak => "~/.var/app/net.lutris.Lutris/data/lutris/runners/wine/",
            Self::DetectAll => "",
        }
    }

    pub fn list_installed_versions(&self) -> Result<Vec<String>, anyhow::Error> {
        list_folders_in_path(self.default_install_dir())
    }
}

// APP_VARIANTS contains the subset of variants of the App enum that are actual apps
pub static APP_VARIANTS: &[App] = &[
    App::Steam,
    App::SteamFlatpak,
    App::Lutris,
    App::LutrisFlatpak,
];

// APP_VARIANTS_WITH_DETECT contains all variants of the App enum including the DetectAll variant
pub static APP_VARIANTS_WITH_DETECT: &[App] = &[
    App::DetectAll,
    App::Steam,
    App::SteamFlatpak,
    App::Lutris,
    App::LutrisFlatpak,
];
