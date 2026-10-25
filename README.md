# Protonup-rs

Lib, CLI and GUI program to automate the installation and update of Linux Gaming Compatibility tools, like ProtonGE, Luxtorpeda, Boxtron and others.

[![Debian package rust-protonup-rs](https://img.shields.io/debian/v/rust-protonup-rs/sid)](https://tracker.debian.org/pkg/rust-protonup-rs)
[![Crates.io](https://img.shields.io/crates/v/protonup-rs)](https://crates.io/crates/protonup-rs)
[![docs.rs](https://docs.rs/libprotonup/badge.svg)](https://docs.rs/libprotonup)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/10372/badge)](https://www.bestpractices.dev/projects/10372)

<p align="center">
  <img src="protonup-rsui/assets/protonup-rs-logo.png" alt="Logo" width="250" />
</p>

[![asciicast](https://asciinema.org/a/QZ97c4yRwQ6YczTliB1ziZy5Z.svg)](https://asciinema.org/a/QZ97c4yRwQ6YczTliB1ziZy5Z)

## Currently supported tools

These are the tools that are currently supported by this project.

| Project Name      | GitHub Repository |
|-------------------|-------------------|
| GEProton          | [GloriousEggroll/proton-ge-custom](https://github.com/GloriousEggroll/proton-ge-custom) |
| Luxtorpeda        | [luxtorpeda-dev/luxtorpeda](https://github.com/luxtorpeda-dev/luxtorpeda) |
| Boxtron           | [dreamer/boxtron](https://github.com/dreamer/boxtron) |
| VKD3D-Proton      | [HansKristian-Work/vkd3d-proton](https://github.com/HansKristian-Work/vkd3d-proton) |
| Lutris-VKD3D      | [lutris/vkd3d](https://github.com/lutris/vkd3d) |
| DXVK              | [doitsujin/dxvk](https://github.com/doitsujin/dxvk) |
| Kron4ek Wine      | [kron4ek/Wine-Builds](https://github.com/kron4ek/Wine-Builds) |
| GEProton-rtsp     | [SpookySkeletons/proton-ge-rtsp](https://github.com/SpookySkeletons/proton-ge-rtsp) |
| Proton-CachyOS    | [CachyOS/proton-cachyos](https://github.com/CachyOS/proton-cachyos) |

Adding new tools should be a simple process, check the docs section below!

## Usage

### Interactive TUI Mode

The default way is to simply invoke the cli, and navigate the text interface.

```bash
protonup-rs
```

### CLI Mode (Non-Interactive)

For automation or scripting, you can use CLI arguments to skip the TUI:

```bash
# Install latest GEProton for Steam (auto-detected)
protonup-rs --tool GEProton

# Install specific version for Lutris
protonup-rs --tool GEProton --version 8.26 --for lutris

# Install to custom path
protonup-rs --tool GEProton --version latest --for ~/.local/steam

# Force overwrite existing installation
protonup-rs --tool GEProton --for steam --force
```

CLI mode fetches release metadata and skips versions already installed at the
target location before downloading any archives. Skipping exits successfully.
Use `--force` to download and reinstall an existing version.

#### CLI Options

```
Options:
  -q, --quick-download     Skip Menu, auto detect apps and download using default parameters
  -f, --force              Force install for existing apps during quick downloads
      --tool <TOOL>        Compatibility tool to install (e.g., GEProton, Luxtorpeda)
      --version <VERSION>  Version to install (use "latest" for the latest version)
      --for <FOR>          Target for installation. Use "steam", "lutris", or a custom path. If omitted, auto-detects based on tool compatibility
  -h, --help               Print help
```

**`--for` argument behavior:**

- `steam` / `Steam` - Install to Steam (Native or Flatpak)
- `lutris` / `Lutris` - Install to Lutris (Native or Flatpak)
- `<path>` - Any other value is treated as a custom installation path
- Omitted - Auto-detects based on the tool's compatible applications and what's installed

---

## Installing (Available for x86 and ARM !)

Using Debian (Forky +), Ubuntu or a derivative ?
The tool might be available in your repository.

`sudo apt install protonup-rs`

### Clickable download (Steam Deck Friendly)

Download the installer `.desktop` file for the app you want, drag it from the Downloads folder to your desktop, and run it (double click).

<table>
  <tr>
    <th align="center">CLI/TUI (<code>protonup-rs</code>)</th>
    <th align="center">GUI (<code>protonup-rsui</code>)</th>
  </tr>
  <tr>
    <td align="center"><a href="https://github.com/auyer/protonup-rs/releases/latest/download/protonup-rs-install.desktop">Download CLI installer</a></td>
    <td align="center"><a href="https://github.com/auyer/protonup-rs/releases/latest/download/protonup-rsui-install.desktop">Download GUI installer</a></td>
  </tr>
</table>

> **NOTE**: These download a simple `.desktop` file that downloads the pre-compiled binary from the release, decompresses it, places it in `$HOME/.local/bin/`, and adds this folder to your PATH.

### Via .deb or .rpm

Download the `.deb` (Debian, Ubuntu and derivatives) or `.rpm` (Fedora, openSUSE and derivatives) package for your architecture from the [latest release](https://github.com/auyer/protonup-rs/releases/latest).

Then, from the folder where you downloaded it, run:

```bash
# Debian / Ubuntu
sudo apt install ./protonup-rs_*.deb
sudo apt install ./protonup-rsui_*.deb

# Fedora / openSUSE
sudo rpm -i protonup-rs_*.rpm
sudo rpm -i protonup-rsui_*.rpm
```

> On ARM systems, pick the `arm64` package instead of `amd64`.

### Via the install script

A single script downloads the binary, desktop entry and icons, and installs everything for you (no `.deb`/`.rpm` needed):

```bash
curl -fsSL https://raw.githubusercontent.com/auyer/protonup-rs/main/scripts/install.sh -o install.sh
bash install.sh
```

By default it installs for the current user (binary to `~/.local/bin`, launcher to `~/.local/share/applications`). Useful options:

```bash
bash install.sh --system     # install system-wide (uses sudo)
bash install.sh --version v1.0.0
bash install.sh --uninstall
bash install.sh --help       # see all options
```

### Or manually (multiple methods available)

The instructions below refer to `amd64` (x86) release. If using ARM, just replace with `arm64`.
Get the latest binary:

Most devices (Intel, AMD, Steam Deck...): [CLI/TUI Download link (x86)](https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-amd64.zip) or [GUI Download link (x86)](https://github.com/auyer/protonup-rs/releases/latest/download/protonup-rsui-linux-amd64.tar.gz)

if using arm (raspberry pi, phones, Asahi users, Qualcomm laptop...):
[CLI/TUI Download link (ARM64)](https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-arm64.zip) or [GUI Download link (ARM64)](https://github.com/auyer/protonup-rs/releases/latest/download/protonup-rsui-linux-arm64.tar.gz)

It is a single binary. You can just run it, or add it to your path so you can call it from anywhere.

Quick way to add it to your path:
or download the zip from the releases page

```
cd Downloads
sudo unzip protonup-rs-linux-amd64.zip -d /usr/bin

sudo unzip protonup-rsui-linux-amd64.zip -d /usr/bin
```

## Building from source

You can install from source using the last released version in Crates.io:

```
cargo install protonup-rs
```

Or clone repo:

```bash
cd protonup-rs
cargo build -p protonup-rs --release
mv ./target/release/protonup-rs "your path"
```

To build the GUI (`protonup-rsui`) instead, replace `protonup-rs` with `protonup-rsui` in the commands above.

## Docs

Code docs available in the [docs/docs.md](./docs/docs.md) file, and default generated docs in [docs.rs/libprotonup](https://docs.rs/libprotonup/latest/libprotonup/).

### Roadmap

The GUI is implemented and functional using the iced framework.
Future improvements may include:

- Additional UI polish and refinements
- More compatibility tools support
- Enhanced error handling and user feedback

## Feedbacks & Contributing

This project accepts contributions and feedbacks from anyone.
For feedbacks, please use GitHub Issues or Discussions. Please be polite ([code-of-conduct](https://www.rust-lang.org/policies/code-of-conduct)).

For contributions, there aren't many rules.
Just try to justify your changes, and try to make the pull request easy to review.
Is is very recommended to add tests, specially for complex code.
Thanks!

### Roles and Maintainers

This project is maintained only by the creator [@auyer](https://rcpassos.me).

The project is open to having other contributors.

> **NOTE**: This has no relations with the original ProtonUp project, and I thankful for the original author.
>
> I've create it because the original project had a few issues with its Python dependencies (that most likely got fixed already).
> I wanted to to re-create it in rust, in a way it could be used as a lib and a CLI.
