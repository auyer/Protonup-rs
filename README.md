# Protonup-rs

Lib, CLI and GUI(wip) program to automate the installation and update of Proton-GE

> **NOTE**: This has no relations with the original ProtonUp project, and I am glad it was created.
> ~~This is not nearly as feature complete as the original Protonup~~.
>
> I've create it because the original project had a few issues with its Python dependencies (that most likely got fixed already).
> I wanted to to re-create it in rust, in a way it could be used as a lib and a CLI.
> ~~If this repo gets to a stable and feature rich state, I will publish it to Cargo and other repositories.~~ I guess it got there! Thanks!

[![asciicast](https://asciinema.org/a/QZ97c4yRwQ6YczTliB1ziZy5Z.svg)](https://asciinema.org/a/QZ97c4yRwQ6YczTliB1ziZy5Z)

## Usage

The default way is to simply invoke the cli, and navigate the text interface.

```bash
protonup-rs
```

To run a quick update and get the latest GE Proton version without navigating the TUI, you can use the quick flag:

```bash
Usage: protonup-rs [OPTIONS]

Options:
  -q, --quick-download  Skip Menu, auto detect apps and download using default parameters
  -h, --help            Print help
```

---

## Installing

### Clickable download (Steam Deck Friendly)

- download .decktop file
- run it
- open a new terminal window to run `protonup-rs`

<h3 align="center">
  <a name="download button" href="https://github.com/auyer/protonup-rs/releases/latest/download/protonup-rs-install.desktop">Click Here to Download installer</a>
</h3>

> **NOTE**: This will download a simple ".desktop" file that will download the pre-compiled binary from release, decompress it, place it in "$HOME/.local/bin/", and add this folder to your PATH.

### In one line

```bash
wget https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-amd64.tar.gz -O - | tar -xz && zenity --password | sudo -S mv protonup-rs /usr/bin/
```

This assumes `/usr/bin` is in your path. You may change this to any other location (in your path `echo $PATH`).

### Or manually

Get the latest binary:
[Download link](https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-amd64.zip)

It is a single binary. You can just run it, or add it to your path so you can call it from anywhere.

Quick way to add it to your path:
or download the zip from the releases page

```
cd Downloads
sudo unzip protonup-rs-linux-amd64.zip -d /usr/bin
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

## GUI

Not ready for usage.

The GUI is in its [early stages](https://github.com/auyer/Protonup-rs/tree/feature/gui). My current plan is to develop it in the iced framework, but GUI development is not my forte.
