use std::{path::PathBuf, fmt};

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Launcher {
    Lutris(LauncherData),
    LutrisFlatpak(LauncherData),
    Steam(LauncherData),
    SteamFlatpak(LauncherData),
}

impl fmt::Display for Launcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Launcher::Lutris(_data) => {"Lutris:"},
            Launcher::LutrisFlatpak(_data) => {"Lutris Flatpak:"},
            Launcher::Steam(_data) => {"Steam:"},
            Launcher::SteamFlatpak(_data) => {"Steam Flatpak:"},
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LauncherData {
    // Location of launcher's runner/wine folder
    pub path: PathBuf,
    pub installs: Vec<Install>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Install {
    name: String,
}
