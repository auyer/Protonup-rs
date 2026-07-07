use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    about = "Protonup-rs Install and Manage Proton/Wine and other Game Runtimes.\n\nRun without arguments to start the interactive TUI mode, or use the options:"
)]
pub struct Opt {
    /// Skip Menu, auto detect apps and download using default parameters
    #[arg(short, long)]
    pub quick_download: bool,

    /// Force install for existing apps during quick downloads
    #[arg(short, long)]
    pub force: bool,

    /// Compatibility tool to install (e.g., GEProton, Luxtorpeda)
    #[arg(long)]
    pub tool: Option<String>,

    /// Version to install (use "latest" for the latest version)
    #[arg(long)]
    pub version: Option<String>,

    /// Target for installation. Use "steam", "lutris", or a custom path.
    /// If omitted, auto-detects Steam or Lutris.
    #[arg(long)]
    pub r#for: Option<String>,

    /// Show release notes for latest versions of default tools
    #[arg(short, long)]
    pub whats_new: bool,
}
