# Protonup-rs

Lib, CLI and GUI(wip) program to automate the installation and update of Proton-GE

> **NOTE**: This is not nearly as feature complete as the original Protonup.
> I've create it because the original project had a few issues with its Python dependencies (that most likely got fixed already). 
> I wanted to to re-create it in rust, in a way it could be used as a lib and a CLI.
> If this repo gets to a stable and feature rich state, I will publish it to Cargo and other repositories.

[![asciicast](https://asciinema.org/a/rEO6Oipjn4rBkTWAtH1IFf3Xe.svg)](https://asciinema.org/a/rEO6Oipjn4rBkTWAtH1IFf3Xe)

## Usage

The default way is to simply invoke the cli, and navigate the text interface.
```bash
protonup-rs
```

To run a quick update and get the latest GE Proton version, you can use the quickUpdate flag `-q`
```bash
protonup-rs -q 
```

## Installing:

Get the latest binary:
[Download link](https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-amd64.zip)

It is a single binary. You can just run it, or add it to your path so you can call it from anywhere.

Quick way to add it to your path:
```
cd Downloads
unzip protonup-rs-linux-amd64.zip -d ~/.local/bin
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

