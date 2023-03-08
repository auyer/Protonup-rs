pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_INSTALL_DIR: &str = "~/.steam/steam/compatibilitytools.d/";
pub const DEFAULT_INSTALL_DIR_FLATPAK: &str =
    "~/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d/";
pub const DEFAULT_LUTRIS_INSTALL_DIR: &str = "~/.local/share/lutris/runners/wine/";
pub const DEFAULT_LUTRIS_INSTALL_DIR_FLATPAK: &str =
    "~/.var/app/net.lutris.Lutris/data/lutris/runners/wine/";
pub const TEMP_DIR: &str = "/tmp/";

pub const GITHUB: &str = "https://api.github.com/repos";
pub const GITHUB_REPO: &str = "proton-ge-custom";
pub const LUTRIS_GITHUB_REPO: &str = "wine-ge-custom";
pub const GITHUB_ACCOUNT: &str = "GloriousEggroll";
pub const USER_AGENT: &str = "protoup-rs";

// pub const CONFIG_FILE: &str = "~/.config/protonup/config.ini";
// use const_format::formatcp;
// pub const USER_AGENT: &'static str =  formatcp!("{}/v{}", USER_AGENT, VERSION);
