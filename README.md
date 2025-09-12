<div align="center">
  <h1>
    <img alt="Topgrade" src="doc/topgrade_transparent.png" width="850px">
  </h1>

  <a href="https://github.com/topgrade-rs/topgrade/releases"><img alt="GitHub Release" src="https://img.shields.io/github/release/topgrade-rs/topgrade.svg"></a>
  <a href="https://crates.io/crates/topgrade"><img alt="crates.io" src="https://img.shields.io/crates/v/topgrade.svg"></a>
  <a href="https://aur.archlinux.org/packages/topgrade"><img alt="AUR" src="https://img.shields.io/aur/version/topgrade.svg"></a>
  <a href="https://formulae.brew.sh/formula/topgrade"><img alt="Homebrew" src="https://img.shields.io/homebrew/v/topgrade.svg"></a>

  <img alt="Demo" src="doc/topgrade_demo.gif">
</div>


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
- Windows: [Chocolatey][choco], [Scoop][scoop] or [Winget][winget]
- PyPi: [pip](https://pypi.org/project/topgrade/)
- Fedora: [Copr](https://copr.fedorainfracloud.org/coprs/lilay/topgrade/)

[choco]: https://community.chocolatey.org/packages/topgrade
[scoop]: https://scoop.sh/#/apps?q=topgrade
[winget]: https://winstall.app/apps/topgrade-rs.topgrade

Other systems users can either use `cargo install` or the compiled binaries from the release page.
The compiled binaries contain a self-upgrading feature.

### Verify downloads

We publish integrity and provenance data with each release:

- `SHA256SUMS` and its Cosign signature (`SHA256SUMS.sig`) and certificate (`SHA256SUMS.crt`)
- Per-file Cosign signatures: `<asset>.sig` and `<asset>.crt`
- SBOM: `sbom.cdx.json` (CycloneDX)

Quick verification:

1. Verify checksums (Linux/macOS):

   ```sh
   sha256sum -c SHA256SUMS  # or: shasum -a 256 -c SHA256SUMS
   ```

   Windows (PowerShell):

   ```powershell
   Get-Content .\SHA256SUMS | ForEach-Object {
     $parts = $_ -split "\s+"; $expected=$parts[0]; $file=$parts[-1]
     $actual = (Get-FileHash -Algorithm SHA256 $file).Hash.ToLower()
     if ($actual -eq $expected) { "OK `t $file" } else { "FAIL`t $file" }
   }
   ```

2. Verify the signed checksum manifest (recommended):

   ```sh
   cosign verify-blob \
     --signature SHA256SUMS.sig \
     --certificate SHA256SUMS.crt \
     SHA256SUMS
   ```

   For stricter checks:

   ```sh
   cosign verify-blob \
     --certificate-oidc-issuer https://token.actions.githubusercontent.com \
     --certificate-identity-regexp 'https://github.com/topgrade-rs/topgrade/.+' \
     --signature SHA256SUMS.sig \
     --certificate SHA256SUMS.crt \
     SHA256SUMS
   ```

3. Verify a single asset (optional):

   ```sh
   cosign verify-blob \
     --signature topgrade-<tag>-<triple>.<ext>.sig \
     --certificate topgrade-<tag>-<triple>.<ext>.crt \
     topgrade-<tag>-<triple>.<ext>
   ```

See `RELEASE_PROCEDURE.md` for more details.

#### Install Cosign

- macOS (Homebrew):

  ```sh
  brew install cosign
  ```

- Windows (Scoop):

  ```powershell
  scoop install cosign
  ```

- Windows (Chocolatey):

  ```powershell
  choco install cosign
  ```

- Linux (distro packages):

  - Debian/Ubuntu (22.04+):

    ```sh
    sudo apt-get update
    sudo apt-get install -y cosign
    ```

  - Fedora/RHEL/CentOS Stream (9+):

    ```sh
    sudo dnf install -y cosign
    ```

  - Arch Linux:

    ```sh
    sudo pacman -S --needed cosign
    ```

  - openSUSE:

    ```sh
    sudo zypper install -y cosign
    ```

  - Alpine:

    ```sh
    sudo apk add cosign
    ```

  - Nix/NixOS:

    ```sh
    nix-env -iA nixpkgs.cosign
    ```

#### View the CycloneDX SBOM

`sbom.cdx.json` is standard CycloneDX JSON. You can:

- Quick look with jq (top-level components):

  ```sh
  jq '.components[] | {name, version, purl, licenses}' sbom.cdx.json
  ```

- Use CycloneDX CLI (pretty-print/validate):

  ```sh
  # macOS (Homebrew)
  brew install cyclonedx/cyclonedx/cyclonedx-cli
  cyclonedx validate --input-file sbom.cdx.json
  cyclonedx view --input-file sbom.cdx.json
  ```

For more viewers and tooling, see the [CycloneDX Tool Center](https://cyclonedx.org/tool-center/).

## Usage

Just run `topgrade`.

## Configuration

See `config.example.toml` for an example configuration file.

## Migration and Breaking Changes

Whenever there is a **breaking change**, the major version number will be bumped,
and we will document these changes in the release note, please take a look at
it when updated to a major release.

> Got a question? Feel free to open an issue or discussion!

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

## Discord server

Welcome to [join](https://discord.gg/Q8HGGWundY) our Discord server if you want
to discuss Topgrade!
