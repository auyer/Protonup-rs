# Protonup-rs

Lib, CLI and GUI program to automate the installation and update of Proton-GE

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

## Building from source

Clone repo

```bash
cd protonup-rs
cargo build -p protonup-rs --release
mv ./target/release/protonup-rs "your path"
```

## GUI

Not ready for usage.
