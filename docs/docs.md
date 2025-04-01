# Protonup-rs Documentation

## Overview

Protonup-rs is a Rust application that helps install and update:

- GE-Proton for Steam
- Wine-GE for Lutris

It provides both a command-line interface (CLI) and a work-in-progress graphical interface (GUI).

## Features

- Automatic detection of Steam/Lutris installations
- Quick update mode for automated updates
- Management of existing Proton installations
- Support for both native and Flatpak versions
- Custom installation locations

## Installation

### Quick Install Methods

1. **Desktop Installer**:
   - Download the [installer .desktop file](https://github.com/auyer/protonup-rs/releases/latest/download/protonup-rs-install.desktop)
   - Run the .desktop as executable
   - Open a terminal to use `protonup-rs`

2. **Command Line**:

```bash
   sh -c 'if curl -S -s -L -O --output-dir /tmp/ --connect-timeout 60 https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-amd64.tar.gz ; then tar -xvzf /tmp/protonup-rs-linux-amd64.tar.gz -C /tmp/ && mv /tmp/protonup-rs ${HOME}/.local/bin/ && [[ "$SHELL" == *"bash"* ]] && [ "$SHELL" = "/bin/bash" ] && echo "export PATH=\"$PATH:${HOME}/.local/bin\"" >> ${HOME}/.bashrc || ([ "$SHELL" = "/bin/zsh" ] && echo "export PATH=\"$PATH:${HOME}/.local/bin\"" >> ${HOME}/.zshrc ) && rm /tmp/protonup-rs-linux-amd64.tar.gz; else echo "Something went wrong, please report this if it is a bug"; read; fi'   ```

### Manual Installation

1. Download the latest binary from [Releases](https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-amd64.zip)
2. Unzip and move to your PATH:

```bash
unzip protonup-rs-linux-amd64.zip -d /usr/local/bin #(or any other path location)
```

### From Source

```bash
# From crates.io
cargo install protonup-rs

# From repository
git clone https://github.com/auyer/protonup-rs
cd protonup-rs
cargo build -p protonup-rs --release
mv ./target/release/protonup-rs /usr/local/bin #(or any other path location)
```

## Usage

### Basic Usage

Run the interactive menu:

```bash
protonup-rs
```

### Quick Update Mode

Automatically detect apps and download updates:

```bash
protonup-rs -q
```

Force install existing apps during quick downloads:

```bash
protonup-rs -q --force
```

### Menu Options

1. **Quick Update**: Auto-detect apps and download
2. **Download for Steam**: Install GE-Proton specifically for Steam
3. **Download for Lutris**: Install Wine-GE/GE-Proton for Lutris
4. **Custom Location**: Install to a custom directory
5. **Manage Installations**: View and manage existing Proton versions

## External Interactions

This software relies on the [github.com/GloriousEggroll/proton-ge-custom](https://github.com/GloriousEggroll/proton-ge-custom) project repository to be available.
We should make it possible to add custom repositories, but this is not necessary at the moment.

## Development

### Building

1. Clone the repository
2. Build with Cargo:

   ```bash
   cargo build --release -p protonup-rs
   ```

### GUI Development

The GUI is currently in early development using the Iced framework. Contributions are welcome!

## Contributing

Contributions are welcome! Please open issues or pull requests on GitHub.
