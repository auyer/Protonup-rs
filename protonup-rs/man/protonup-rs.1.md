% PROTONUP-RS(1) | User Commands

# NAME

protonup-rs - install and manage Proton/Wine and other game runtimes

# SYNOPSIS

`protonup-rs` [OPTIONS]

# DESCRIPTION

**protonup-rs** is a program to automate the installation and update of
Linux Gaming Compatibility tools, such as GEProton, Luxtorpeda, Boxtron,
and others.

Run without arguments to start the interactive TUI mode.
Use the options below for non-interactive (CLI) operation.

# OPTIONS

  * `-q`, `--quick-download`:
    Skip the menu, auto-detect installed applications (Steam, Lutris),
    and download the latest versions of their default compatibility tools.

  * `-f`, `--force`:
    Force re-installation for already existing tools during quick downloads.

  * `--tool` _TOOL_:
    Compatibility tool to install. Case-insensitive name.
    Supported tools:

    **GEProton** (Steam, Lutris), **Luxtorpeda** (Steam),
    **Boxtron** (Steam), **VKD3D-Proton** (Lutris),
    **Lutris-VKD3D** (Lutris), **DXVK** (Lutris),
    **Kron4ek Wine** (Lutris), **GEProton RTSP** (Steam),
    **Proton CachyOS** (Steam, Lutris)

  * `--version` _VERSION_:
    Version to install. Use **latest** for the latest release.
    Only used together with `--tool`.

  * `--for` _TARGET_:
    Target for installation. Accepted values:

    **steam**, **Steam**  - Install to Steam (Native or Flatpak)
    **lutris**, **Lutris**  - Install to Lutris (Native or Flatpak)
    _path_                - Any other value is treated as a custom
                            installation path (supports `~` expansion)

    If omitted, the target is auto-detected based on the tool's
    compatible applications and what is installed.

  * `-w`, `--whats-new`:
    Show release notes for the latest versions of default tools.
    When combined with `-q`, shows release notes before and after downloading.
    When combined with `--tool`, shows release notes for the selected tool
    and then proceeds with the installation.

  * `-h`, `--help`:
    Print help and exit.

# INSTALL DIRECTORIES

  **Steam (Native):**
    `~/.steam/steam/compatibilitytools.d/`

  **Steam (Flatpak):**
    `~/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d/`

  **Lutris (Native):**
    Wine-based tools: `~/.local/share/lutris/runners/wine/`
    Runtime tools:   `~/.local/share/lutris/runtime/`

  **Lutris (Flatpak):**
    Wine-based tools: `~/.var/app/net.lutris.Lutris/data/lutris/runners/wine/`
    Runtime tools:   `~/.var/app/net.lutris.Lutris/data/lutris/runtime/`

  **Custom:**
    The provided path is used directly (extracted in-place).

# EXAMPLES

  **TUI mode (interactive):**
    `protonup-rs`

  **Quick update (auto-detect and download):**
    `protonup-rs -q`

  **Quick update, force re-install, and show release notes:**
    `protonup-rs -q -f -w`

  **Install latest GEProton for Steam (auto-detected):**
    `protonup-rs --tool GEProton`

  **Install specific version for Lutris:**
    `protonup-rs --tool GEProton --version 8.26 --for lutris`

  **Install to a custom path:**
    `protonup-rs --tool GEProton --version latest --for ~/.local/steam`

  **Force overwrite existing installation:**
    `protonup-rs --tool GEProton --for steam --force`

  **Check release notes without downloading:**
    `protonup-rs --whats-new`

# ENVIRONMENT

  **HOME:**
    Used implicitly for resolving `~` in custom paths.

    **protonup-rs** does not read any other environment variables for
    configuration.

# TEMPORARY FILES

  Downloads are extracted to standard temporary directories.
  If `/tmp` has insufficient space, the fallback is:
  `~/.local/state/protonup-rs/tmp/`

  These are cleaned up automatically on exit.

# SEE ALSO

  **Project homepage:** https://github.com/auyer/protonup-rs
  **API documentation:** https://docs.rs/libprotonup

  Related tools and projects:

  * GEProton: https://github.com/GloriousEggroll/proton-ge-custom
  * Luxtorpeda: https://github.com/luxtorpeda-dev/luxtorpeda
  * Boxtron: https://github.com/dreamer/boxtron
  * VKD3D-Proton: https://github.com/HansKristian-Work/vkd3d-proton
  * DXVK: https://github.com/doitsujin/dxvk
  * Kron4ek Wine: https://github.com/kron4ek/Wine-Builds
  * GEProton RTSP: https://github.com/SpookySkeletons/proton-ge-rtsp
  * Proton CachyOS: https://github.com/CachyOS/proton-cachyos

# AUTHOR

  Maintained by **Rafael Passos** (@auyer) https://rcpassos.me

# BUGS

  Report issues at: https://github.com/auyer/protonup-rs/issues
