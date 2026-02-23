# Protonup-rs

Lib, CLI and GUI(wip) program to automate the installation and update of Linux Gaming Compatibility tools, like ProtonGE, Luxtorpeda, Boxtron and others.

[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/10372/badge)](https://www.bestpractices.dev/projects/10372)

> **NOTE**: This has no relations with the original ProtonUp project, and I thankful for the original author.
>
> I've create it because the original project had a few issues with its Python dependencies (that most likely got fixed already).
> I wanted to to re-create it in rust, in a way it could be used as a lib and a CLI.

[![asciicast](https://asciinema.org/a/QZ97c4yRwQ6YczTliB1ziZy5Z.svg)](https://asciinema.org/a/QZ97c4yRwQ6YczTliB1ziZy5Z)

## Currently supported tools

These are the tools that are currently supported by this project.

| Project Name      | GitHub Repository |
|-------------------|-------------------|
| GEProton          | [GloriousEggroll/proton-ge-custom](https://github.com/GloriousEggroll/proton-ge-custom) |
| WineGE            | [GloriousEggroll/wine-ge-custom](https://github.com/GloriousEggroll/wine-ge-custom) |
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

- download .desktop file
- run it
- open a new terminal window to run `protonup-rs`

<h3 align="center">
  <a name="download button" href="https://github.com/auyer/protonup-rs/releases/latest/download/protonup-rs-install.desktop">Click Here to Download installer</a>
</h3>

> **NOTE**: This will download a simple ".desktop" file that will download the pre-compiled binary from release, decompress it, place it in "$HOME/.local/bin/", and add this folder to your PATH.

### In one line

Does the same as the installer above, but from the command line

```bash
sh -c 'if curl -S -s -L -O --output-dir /tmp/ --connect-timeout 60 https://github.com/auyer/Protonup-rs/releases/latest/download/protonup-rs-linux-amd64.tar.gz ; then tar -xvzf /tmp/protonup-rs-linux-amd64.tar.gz -C /tmp/ && mv /tmp/protonup-rs ${HOME}/.local/bin/ && [[ "$SHELL" == *"bash"* ]] && [ "$SHELL" = "/bin/bash" ] && echo "export PATH=\"$PATH:${HOME}/.local/bin\"" >> ${HOME}/.bashrc || ([ "$SHELL" = "/bin/zsh" ] && echo "export PATH=\"$PATH:${HOME}/.local/bin\"" >> ${HOME}/.zshrc ) && rm /tmp/protonup-rs-linux-amd64.tar.gz; else echo "Something went wrong, please report this if it is a bug"; read; fi'
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

## Docs

Code docs available in the [docs/docs.md](./docs/docs.md) file, and default generated docs in [docs.rs/libprotonup](https://docs.rs/libprotonup/latest/libprotonup/).

### Roadmap

1. GUI:
  Not ready for usage. I made a few experiments with different libs, and decided to use the iced/cosmic framework.
  I left the [base structure](https://github.com/auyer/Protonup-rs/tree/feature/gui), and hope to get to it someday or to receive contributions.

## Feedbacks & Contributing

This project accepts contributions and feedbacks from anyone.
For feedbacks, please use GitHub Issues or Discussions. Please be polite ([code-of-conduct](https://www.rust-lang.org/policies/code-of-conduct)).

For contributions, there aren't many rules.
Just try to justify your changes, and try to make the pull request easy to review.
Is is very recommended to add tests, specially for complex code.
Thanks!

### Roles and Maintainers

This project is maintained only by the creator [@auyer](https://rcpassos.me)

Maintaining this project does not take a lot of effort.
The project is open to having other contributors, specially if implementing a GUI.
