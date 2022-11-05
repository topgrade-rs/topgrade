<div align="center">
  <img alt="Topgrade" src="doc/topgrade.png" width="850px">
  
  <!--
  <a href="https://github.com/topgrade-rs/topgrade/releases"><img alt="GitHub Release" src="https://img.shields.io/github/release/r-darwish/topgrade.svg"></a>
  <a href="https://crates.io/crates/topgrade"><img alt="crates.io" src="https://img.shields.io/crates/v/topgrade.svg"></a>
  <a href="https://aur.archlinux.org/packages/topgrade"><img alt="AUR" src="https://img.shields.io/aur/version/topgrade.svg"></a>
  <a href="https://formulae.brew.sh/formula/topgrade"><img alt="Homebrew" src="https://img.shields.io/homebrew/v/topgrade.svg"></a>
  -->  

  <img alt="Demo" src="doc/screenshot.gif" width="550px">
</div>
  
## Introduction

> **Note**
> This is a fork of [topgrade by r-darwish](https://github.com/r-darwish/topgrade) to keep it maintained.

Keeping your system up to date usually involves invoking multiple package managers.
This results in big, non-portable shell one-liners saved in your shell.
To remedy this, **Topgrade** detects which tools you use and runs the appropriate commands to update them.

## Installation

- Arch Linux: [AUR](https://aur.archlinux.org/packages/topgrade) package.
- NixOS: _topgrade_ package in `nixpkgs`.
- macOS: [Homebrew](https://formulae.brew.sh/formula/topgrade) or [MacPorts](https://ports.macports.org/port/topgrade/).

Other systems users can either use `cargo install` or use the compiled binaries from the release page.
The compiled binaries contain a self-upgrading feature.

Topgrade requires Rust 1.51 or above.

## Documentation

> **Warning**
> Work in Progress

You can visit the documentation at [topgrade-rs.github.io](https://topgrade-rs.github.io/) .

## Usage

Just run `topgrade`.
See [the documentation](https://topgrade-rs.github.io/) for the list of things Topgrade supports.

## Customization

See `config.example.toml` for an example configuration file.

### Configuration path

The configuration should be placed in the following paths depending by the operating system:

- **Windows** - `%APPDATA%/topgrade.toml`
- **macOS** and **other Unix systems** - `${XDG_CONFIG_HOME:-~/.config}/topgrade.toml`

## Contribution

### Problems or missing features?

Open a new Issue describing your problem and if possible with a possible solution.

### Missing a feature or found an unsupported tool/distro?

Just let us now what you are missing by opening an issue.
For tools please open an Issue describing the tool, which platforms it supports and if possible, give us an example of its usage.

### Want to contribute to the code?

Just fork the repository and start coding.

### Contribution Guidelines

- Check if your code passes `cargo fmt` and `cargo clippy`.
- Check if your code is self explanatory, if not it should be documented by comments.
- Make a Pull Request to the dev branch for new features or to the bug-fixes branch for bug fixes.

## Remote execution

You can specify a key called `remote_topgrades` in the configuration file.
This key should contain a list of hostnames that have Topgrade installed on them.
Topgrade will use `ssh` to run `topgrade` on remote hosts before acting locally.
To limit the execution only to specific hosts use the `--remote-host-limit` parameter.

## ToDo

- [ ] Add a proper testing framework to the code base.
- [ ] Add unit tests for package managers.
- [ ] Split up code into more maintainable parts, eg. putting every linux package manager in a own submodule of linux.rs.
