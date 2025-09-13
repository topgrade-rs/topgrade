# Topgrade

![Topgrade](doc/topgrade_transparent.png)

[![GitHub Release](https://img.shields.io/github/release/topgrade-rs/topgrade.svg)](https://github.com/topgrade-rs/topgrade/releases)
[![crates.io](https://img.shields.io/crates/v/topgrade.svg)](https://crates.io/crates/topgrade)
[![AUR](https://img.shields.io/aur/version/topgrade.svg)](https://aur.archlinux.org/packages/topgrade)
[![Homebrew](https://img.shields.io/homebrew/v/topgrade.svg)](https://formulae.brew.sh/formula/topgrade)
[![Docs & Scripts CI](https://github.com/topgrade-rs/topgrade/actions/workflows/docs_and_scripts.yml/badge.svg)](https://github.com/topgrade-rs/topgrade/actions/workflows/docs_and_scripts.yml)

![Demo](doc/topgrade_demo.gif)
## Introduction

> [!NOTE] This is a fork of [topgrade by r-darwish](https://github.com/r-darwish/topgrade) to keep it maintained.

Keeping your system up to date usually involves invoking multiple package managers. This results in big, non-portable
shell one-liners saved in your shell. To remedy this, **Topgrade** detects which tools you use and runs the appropriate
commands to update them.

## Installation

[![Packaging status](https://repology.org/badge/vertical-allrepos/topgrade.svg)](https://repology.org/project/topgrade/versions)

- Arch Linux: [AUR](https://aur.archlinux.org/packages/topgrade)
- NixOS: [Nixpkgs](https://search.nixos.org/packages?show=topgrade)
- Void Linux: [XBPS](https://voidlinux.org/packages/?arch=x86_64&q=topgrade)
- macOS: [Homebrew](https://formulae.brew.sh/formula/topgrade) or [MacPorts](https://ports.macports.org/port/topgrade/)
- Windows: [Chocolatey][choco], [Scoop][scoop] or [Winget][winget]
- PyPi: [pip](https://pypi.org/project/topgrade/)
- Fedora: [Copr](https://copr.fedorainfracloud.org/coprs/lilay/topgrade/)

[choco]: https://community.chocolatey.org/packages/topgrade
[scoop]: https://scoop.sh/#/apps?q=topgrade
[winget]: https://winstall.app/apps/topgrade-rs.topgrade

Users of other systems can either use `cargo install` or the compiled binaries from the release page. The compiled
binaries contain a self-upgrading feature.

## Usage

Just run `topgrade`.

## Configuration

See `config.example.toml` for an example configuration file.

## Migration and Breaking Changes

Whenever there is a breaking change, the major version number will be bumped, and we will document these changes in the
release notes. Please review them when updating to a major release.

> Got a question? Feel free to open an issue or discussion!

### Configuration Path

#### `CONFIG_DIR` on each platform

- **Windows**: `%APPDATA%`
- **macOS** and **other Unix systems**: `${XDG_CONFIG_HOME:-~/.config}`

Topgrade will look for the configuration file in the following places, in order of priority:

1. `CONFIG_DIR/topgrade.toml`
2. `CONFIG_DIR/topgrade/topgrade.toml`

If the file with higher priority is present—regardless of whether it is valid—the other configuration files will be
ignored.

On the first run (no configuration file exists), Topgrade will create a configuration file at `CONFIG_DIR/topgrade.toml`
for you.

### Custom Commands

Custom commands can be defined in the config file which can be run before, during, or after the inbuilt commands, as
required. By default, custom commands are run using a new shell according to the `$SHELL` environment variable on Unix
(falls back to `sh`) or `pwsh` on Windows (falls back to `powershell`).

On Unix, if you want to run your command using an interactive shell (for example, to source your shell's rc files), you
can add `-i` at the start of your custom command. But note that this requires the command to exit the shell correctly or
else the shell will hang indefinitely.

## Remote Execution

You can specify a key called `remote_topgrades` in the configuration file. This key should contain a list of hostnames
that have Topgrade installed on them. Topgrade will use `ssh` to run `topgrade` on remote hosts before acting locally.
To limit the execution only to specific hosts use the `--remote-host-limit` parameter.

## Contribution

### Problems or missing features?

Open a new issue describing your problem and if possible provide a solution.

### Missing a feature or found an unsupported tool/distro?

Just let us know what you are missing by opening an issue. For tools, please open an issue describing the tool, which
platforms it supports and if possible, give us an example of its usage.

### Want to contribute to the code?

Just fork the repository and start coding.

### Contribution Guidelines

See [CONTRIBUTING.md](https://github.com/topgrade-rs/topgrade/blob/main/CONTRIBUTING.md).

## Roadmap

- [ ] Add a proper testing framework to the code base.
- [ ] Add unit tests for package managers.
- [ ] Split up code into more maintainable parts, eg. putting every linux package manager in a own submodule of
      linux.rs.

## Discord server

Welcome to [join](https://discord.gg/Q8HGGWundY) our Discord server if you want to discuss Topgrade!
