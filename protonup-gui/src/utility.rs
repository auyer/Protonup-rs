use std::path::PathBuf;

pub struct Launchers {
    pub steam: Option<Launcher>,
    pub steam_flatpak: Option<Launcher>,
    pub lutris: Option<Launcher>,
    pub lutris_flatpak: Option<Launcher>,
}

pub struct Launcher {
    // Location of launcher's runner/wine folder
    path: PathBuf,
    installs: Vec<Install>,
}

struct Install {
    name: String,
}
