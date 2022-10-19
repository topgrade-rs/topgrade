![Topgrade](doc/topgrade.png)
<!---
![GitHub release](https://img.shields.io/github/release/r-darwish/topgrade.svg)
[![Crates.io](https://img.shields.io/crates/v/topgrade.svg)](https://crates.io/crates/topgrade)
[![AUR](https://img.shields.io/aur/version/topgrade.svg)](https://aur.archlinux.org/packages/topgrade/)
![homebrew](https://img.shields.io/homebrew/v/topgrade.svg) -->
--->

![Demo](doc/screenshot.gif)

## Fork
This is a fork of [topgrade by r-darwish](https://github.com/r-darwish/topgrade) to keep it maintained.


Keeping your system up to date usually involves invoking multiple package managers.
This results in big, non-portable shell one-liners saved in your shell.
To remedy this, _topgrade_ detects which tools you use and runs the appropriate commands to update them.

## Installation
- Arch Linux: [AUR](https://aur.archlinux.org/packages/topgrade/) package.
- NixOS: _topgrade_ package in `nixpkgs`.
- macOS: [Homebrew](https://brew.sh/) or [MacPorts](https://www.macports.org/install.php).

Other systems users can either use `cargo install` or use the compiled binaries from the release page.
The compiled binaries contain a self-upgrading feature.

Topgrade requires Rust 1.51 or above.

## Documentation[WIP]
You can visit the documentation at [topgrade-rs.github.io](https://topgrade-rs.github.io/) .

## Usage
Just run `topgrade`.
See [the wiki](https://github.com/r-darwish/topgrade/wiki/Step-list) for the list of things Topgrade supports.

## Customization
See `config.example.toml` for an example configuration file.

### Configuration path

The configuration should be placed in the following paths depending by the operating system:

* **Windows** - `%APPDATA%/topgrade.toml`
* **macOS** and **other Unix systems** - `${XDG_CONFIG_HOME:-~/.config}/topgrade.toml`

## Contribution
### Problems or missing features?
Open a new Issue describing your problem and if possible with a possible solution.
### Missing a feature or found an unsupported tool/distro?
Just let us now what you are missing by opening an issue.
For tools please open an Issue describing the tool, which platforms it supports and if possible, give us an example of its usage.
### Want to contribute to the code?
Just fork the repository and start coding. Please let PRs with bug fixes target the staging branch and PRs with new features target the dev branch.

## Remote execution
You can specify a key called `remote_topgrades` in the configuration file.
This key should contain a list of hostnames that have topgrade installed on them.
Topgrade will use `ssh` to run `topgrade` on remote hosts before acting locally.
To limit the execution only to specific hosts use the `--remote-host-limit` parameter.
