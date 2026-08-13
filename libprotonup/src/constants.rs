use super::architecture::CpuArch;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default architecture assumed for release assets that do not embed an
/// architecture suffix (e.g. `GE-Proton11-2.tar.gz`).
/// TODO: when creating a config file, this should be a parameter
pub const DEFAULT_ARCH: CpuArch = CpuArch::X86;

pub const DEFAULT_STEAM_TOOL: &str = "GEProton";
pub const DEFAULT_LUTRIS_TOOL: &str = "GEProton";

pub const USER_AGENT: &str = "protoup-rs";

pub const MIN_TEMP_SPACE_BYTES: u64 = 1_073_741_824; // 1GB
pub const FALLBACK_TEMP_DIR: &str = ".local/state/protonup-rs/tmp";

// pub const CONFIG_FILE: &str = "~/.config/protonup/config.ini";
// use const_format::formatcp;
// pub const USER_AGENT: &'static str =  formatcp!("{}/v{}", USER_AGENT, VERSION);
