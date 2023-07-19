<div align="center">
  <h1>
    <img alt="Topgrade" src="doc/topgrade_transparent.png" width="850px">
  </h1>
  
  <a href="https://github.com/topgrade-rs/topgrade/releases"><img alt="GitHub Release" src="https://img.shields.io/github/release/topgrade-rs/topgrade.svg"></a>
  <a href="https://crates.io/crates/topgrade"><img alt="crates.io" src="https://img.shields.io/crates/v/topgrade.svg"></a>
  <a href="https://aur.archlinux.org/packages/topgrade"><img alt="AUR" src="https://img.shields.io/aur/version/topgrade.svg"></a>
  <a href="https://formulae.brew.sh/formula/topgrade"><img alt="Homebrew" src="https://img.shields.io/homebrew/v/topgrade.svg"></a>

  <img alt="Demo" src="doc/screenshot.gif" width="550px">
</div>

## Maintainers Wanted

I currently have not enough time to maintain this project on the level required and which the project deserves. For this reason I'm asking the community to help supporting the project, to help and work on resolving issues and create new features. Thanks for all your help.


## Introduction

> **Note**
> This is a fork of [topgrade by r-darwish](https://github.com/r-darwish/topgrade) to keep it maintained.

Keeping your system up to date usually involves invoking multiple package managers.
This results in big, non-portable shell one-liners saved in your shell.
To remedy this, **Topgrade** detects which tools you use and runs the appropriate commands to update them.

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/topgrade.svg)](https://repology.org/project/topgrade/versions)

- Arch Linux: [AUR](https://aur.archlinux.org/packages/topgrade)
- NixOS: [Nixpkgs](https://search.nixos.org/packages?show=topgrade)
- Void Linux: [XBPS](https://voidlinux.org/packages/?arch=x86_64&q=topgrade)
- macOS: [Homebrew](https://formulae.brew.sh/formula/topgrade) or [MacPorts](https://ports.macports.org/port/topgrade/)
- Windows: [Scoop](https://github.com/ScoopInstaller/Main/blob/master/bucket/topgrade.json)
- PyPi: [pip](https://pypi.org/project/topgrade/)

Other systems users can either use `cargo install` or the compiled binaries from the release page.
The compiled binaries contain a self-upgrading feature.

> Currently, Topgrade requires Rust 1.65 or above. In general, Topgrade tracks 
> the latest stable toolchain.

## Usage

Just run `topgrade`.

Visit the documentation at [topgrade-rs.github.io](https://topgrade-rs.github.io/) for more information.

> **Warning**
> Work in Progress

## Configuration 

See `config.example.toml` for an example configuration file.

### Configuration Path

#### `CONFIG_DIR` on each platform
- **Windows**: `%APPDATA%`
- **macOS** and **other Unix systems**: `${XDG_CONFIG_HOME:-~/.config}`

`topgrade` will look for the configuration file in the following places, in order of priority:

1. `CONFIG_DIR/topgrade.toml`
2. `CONFIG_DIR/topgrade/topgrade.toml`

If the file with higher priority is present, no matter it is valid or not, the other configuration files will be ignored.

On the first run(no configuration file exists), `topgrade` will create a configuration file at `CONFIG_DIR/topgrade.toml` for you.

### Custom Commands

Custom commands can be defined in the config file which can be run before, during, or after the inbuilt commands, as required.
By default, the custom commands are run using a new shell according to the `$SHELL` environment variable on unix (falls back to `sh`) or `pwsh` on windows (falls back to `powershell`).

On unix, if you want to run your command using an interactive shell, for example to source your shell's rc files, you can add `-i` at the start of your custom command.
But note that this requires the command to exit the shell correctly or else the shell will hang indefinitely.

## Remote Execution

You can specify a key called `remote_topgrades` in the configuration file.
This key should contain a list of hostnames that have Topgrade installed on them.
Topgrade will use `ssh` to run `topgrade` on remote hosts before acting locally.
To limit the execution only to specific hosts use the `--remote-host-limit` parameter.

## Contribution

### Problems or missing features?

Open a new issue describing your problem and if possible provide a solution.

### Missing a feature or found an unsupported tool/distro?

Just let us now what you are missing by opening an issue.
For tools, please open an issue describing the tool, which platforms it supports and if possible, give us an example of its usage.

### Want to contribute to the code?

Just fork the repository and start coding.

### Contribution Guidelines

See [CONTRIBUTING.md](https://github.com/topgrade-rs/topgrade/blob/master/CONTRIBUTING.md)

## Roadmap

- [ ] Add a proper testing framework to the code base.
- [ ] Add unit tests for package managers.
- [ ] Split up code into more maintainable parts, eg. putting every linux package manager in a own submodule of linux.rs.
