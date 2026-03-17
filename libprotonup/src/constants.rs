pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_STEAM_TOOL: &str = "GEProton";
pub const DEFAULT_LUTRIS_TOOL: &str = "WineGE";

pub const USER_AGENT: &str = "protoup-rs";

pub const MIN_TEMP_SPACE_BYTES: u64 = 1_073_741_824; // 1GB
pub const FALLBACK_TEMP_DIR: &str = ".local/state/protonup-rs/tmp";

// pub const CONFIG_FILE: &str = "~/.config/protonup/config.ini";
// use const_format::formatcp;
// pub const USER_AGENT: &'static str =  formatcp!("{}/v{}", USER_AGENT, VERSION);
