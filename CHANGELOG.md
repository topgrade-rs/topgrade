# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [16.2.1](https://github.com/topgrade-rs/topgrade/compare/v16.2.0...v16.2.1) - 2025-11-10

### Fixed

- *(release)* Fix .deb distribution ([#1458](https://github.com/topgrade-rs/topgrade/pull/1458))

## [16.2.0](https://github.com/topgrade-rs/topgrade/compare/v16.1.2...v16.2.0) - 2025-11-10

### Added

- *(mise)* run `mise self-update` ([#1450](https://github.com/topgrade-rs/topgrade/pull/1450))
- *(falconf)* add falconf step ([#1219](https://github.com/topgrade-rs/topgrade/pull/1219))
- *(hyprpm)* add hyprpm step ([#1213](https://github.com/topgrade-rs/topgrade/pull/1213))
- *(doom)* add doom.aot option ([#1214](https://github.com/topgrade-rs/topgrade/pull/1214))
- add show_distribution_summary config option ([#1259](https://github.com/topgrade-rs/topgrade/pull/1259))
- *(rustup)* add rustup.channels config ([#1206](https://github.com/topgrade-rs/topgrade/pull/1206))
- *(os)* add AOSC OS support ([#1424](https://github.com/topgrade-rs/topgrade/pull/1424))
- add damp run type ([#1217](https://github.com/topgrade-rs/topgrade/pull/1217))

### Fixed

- *(release)* fix homebrew releases by migrating to dawidd6/action-homebrew-bump-formula ([#1457](https://github.com/topgrade-rs/topgrade/pull/1457))
- *(mise)* fix mise self-update failing when installed via a package manager ([#1456](https://github.com/topgrade-rs/topgrade/pull/1456))
- *(release)* Add man page to .deb distribution ([#1455](https://github.com/topgrade-rs/topgrade/pull/1455))
- *(self-update)* fix windows self-update reporting failure on successful self-update ([#1452](https://github.com/topgrade-rs/topgrade/pull/1452))
- *(pkgfile)* make pkgfile opt-in ([#1449](https://github.com/topgrade-rs/topgrade/pull/1449))
- *(vcpkg)* fix permission denied when updating vcpkg if it's installed as root ([#1447](https://github.com/topgrade-rs/topgrade/pull/1447))
- *(zh_TW)* fixed zh_TW strings ([#1446](https://github.com/topgrade-rs/topgrade/pull/1446))
- *(git)* fix shellexpand::tilde in git_repos in topgrade.d/* ([#1223](https://github.com/topgrade-rs/topgrade/pull/1223))
- *(auto-cpufreq)* skip when install script is not used ([#1215](https://github.com/topgrade-rs/topgrade/pull/1215))
- *(vim)* change nvimrc base_dir for windows ([#1433](https://github.com/topgrade-rs/topgrade/pull/1433))
- *(guix)* fix overcomplicated Guix step ([#1290](https://github.com/topgrade-rs/topgrade/pull/1290))
- *(gem)* fix incorrectly placed debug message in `gem` step ([#1212](https://github.com/topgrade-rs/topgrade/pull/1212))
- *(conda)* replace deprecated `auto_activate_base` ([#1158](https://github.com/topgrade-rs/topgrade/pull/1158))
- *(containers)* fix panic in `containers` step ([#1150](https://github.com/topgrade-rs/topgrade/pull/1150))
- *(jetbrains-toolbox)* fix step not dry running ([#1253](https://github.com/topgrade-rs/topgrade/pull/1253))

### Other

- comment run_config_update ([#1448](https://github.com/topgrade-rs/topgrade/pull/1448))
- Expand LLM guidelines in CONTRIBUTING.md ([#1445](https://github.com/topgrade-rs/topgrade/pull/1445))
- Add AI guidelines to CONTRIBUTING.md ([#1444](https://github.com/topgrade-rs/topgrade/pull/1444))
- add comments to Config::allowed_steps ([#1291](https://github.com/topgrade-rs/topgrade/pull/1291))
- *(nix)* Deduplicate run_nix and run_nix_self_upgrade nix --version checking ([#1376](https://github.com/topgrade-rs/topgrade/pull/1376))
- remove commented-out library code and unnecessary bin declaration ([#1373](https://github.com/topgrade-rs/topgrade/pull/1373))
- Simplify target cfgs ([#1346](https://github.com/topgrade-rs/topgrade/pull/1346))
- tidy up binary-conflict code ([#1329](https://github.com/topgrade-rs/topgrade/pull/1329))
- Improve installation section ([#1442](https://github.com/topgrade-rs/topgrade/pull/1442))
- *(deps)* Update jetbrains-toolbox-updater ([#1438](https://github.com/topgrade-rs/topgrade/pull/1438))
- remove template expansion in code contexts ([#1434](https://github.com/topgrade-rs/topgrade/pull/1434))
- *(deps)* bump github/codeql-action from 4.31.0 to 4.31.2 ([#1427](https://github.com/topgrade-rs/topgrade/pull/1427))
- don't persist credentials in actions/checkout ([#1422](https://github.com/topgrade-rs/topgrade/pull/1422))
- Improve CONTRIBUTING.md ([#1420](https://github.com/topgrade-rs/topgrade/pull/1420))
- Update SECURITY.md ([#1421](https://github.com/topgrade-rs/topgrade/pull/1421))
- Enforce conventional commits in PR titles ([#1418](https://github.com/topgrade-rs/topgrade/pull/1418))
- Improve contributing section
- Remove roadmap
- Reformat README.md
- Update installation methods
- *(release)* Fix dispatch error in create_release_assets.yml ([#1406](https://github.com/topgrade-rs/topgrade/pull/1406))

## [16.1.2](https://github.com/topgrade-rs/topgrade/compare/v16.1.1...v16.1.2) - 2025-11-01

### Fixed

- *(release)* Fix cross-compilation for arm requiring glibc>=2.39 ([#1405](https://github.com/topgrade-rs/topgrade/pull/1405))
- *(release)* Fix FreeBSD build ([#1404](https://github.com/topgrade-rs/topgrade/pull/1404))
- *(release)* Fix FreeBSD build ([#1402](https://github.com/topgrade-rs/topgrade/pull/1402))
- *(release)* Fix manual workflow trigger ([#1401](https://github.com/topgrade-rs/topgrade/pull/1401))
- *(release)* Fix FreeBSD build and add manual workflow trigger ([#1399](https://github.com/topgrade-rs/topgrade/pull/1399))

### Other

- *(release)* Fix cross trying to fmt ([#1403](https://github.com/topgrade-rs/topgrade/pull/1403))

## [16.1.1](https://github.com/topgrade-rs/topgrade/compare/v16.1.0...v16.1.1) - 2025-11-01

### Fixed

- *(typst)* Skip typst when self-update is disabled ([#1397](https://github.com/topgrade-rs/topgrade/pull/1397))
- *(release)* Fix winget release workflow ([#1395](https://github.com/topgrade-rs/topgrade/pull/1395))
- *(release)* Fix FreeBSD release ([#1393](https://github.com/topgrade-rs/topgrade/pull/1393))
- *(release)* Fix FreeBSD release ([#1391](https://github.com/topgrade-rs/topgrade/pull/1391))

### Other

- Update from deprecated macos-13 to macos-15-intel ([#1394](https://github.com/topgrade-rs/topgrade/pull/1394))

## [16.1.0](https://github.com/topgrade-rs/topgrade/compare/v16.0.4...v16.1.0) - 2025-10-31

### Added

- *(deb-get)* Skip non-deb-get packages by passing --dg-only ([#1386](https://github.com/topgrade-rs/topgrade/pull/1386))
- *(typst)* add typst step ([#1374](https://github.com/topgrade-rs/topgrade/pull/1374))
- *(step)* Add atuin step ([#1367](https://github.com/topgrade-rs/topgrade/pull/1367))
- *(nix)* support upgrading Determinate Nix ([#1366](https://github.com/topgrade-rs/topgrade/pull/1366))
- *(sudo)* print warning if Windows Sudo is misconfigured
- *(sudo)* print warning if steps were skipped due to missing sudo
- *(sudo)* add SudoKind::Null
- detect and warn if running as root
- add `--no-tmux` flag ([#1328](https://github.com/topgrade-rs/topgrade/pull/1328))
- add step for mandb - user and system (update man entries) ([#1319](https://github.com/topgrade-rs/topgrade/pull/1319))
- support for pkgfile ([#1306](https://github.com/topgrade-rs/topgrade/pull/1306))
- add "show_skipped" option in config file #1280 ([#1286](https://github.com/topgrade-rs/topgrade/pull/1286))
- fix typos ([#1221](https://github.com/topgrade-rs/topgrade/pull/1221))
- *(conda)* allow configuring additional envs to update ([#1048](https://github.com/topgrade-rs/topgrade/pull/1048))
- *(step)* nix-helper ([#1045](https://github.com/topgrade-rs/topgrade/pull/1045))
- *(winget)* winget uses sudo when `[windows] winget_use_sudo = true` ([#1061](https://github.com/topgrade-rs/topgrade/pull/1061))
- suppress pixi release notes by default ([#1225](https://github.com/topgrade-rs/topgrade/pull/1225))

### Fixed

- *(freshclam)* run with sudo when running without sudo fails ([#1118](https://github.com/topgrade-rs/topgrade/pull/1118))
- *(tldr)* move tldr to be a generic step ([#1370](https://github.com/topgrade-rs/topgrade/pull/1370))
- *(nix)* fix nix upgrade command selection for profiles in XDG_STATE_HOME ([#1354](https://github.com/topgrade-rs/topgrade/pull/1354))
- *(containers)* Docker update fails on M Macs due to platform / ([#1360](https://github.com/topgrade-rs/topgrade/pull/1360))
- *(sudo)* reorder require_sudo() after print_separator()
- *(sudo)* use require_sudo for windows commands
- *(sudo)* prevent sudo_command = "sudo" finding gsudo
- *(sudo)* set sudo flags depending on kind
- skip gcloud update step if component manager is disabled ([#1237](https://github.com/topgrade-rs/topgrade/pull/1237))
- *(i18n)* use double-quotes for translations with newlines
- *(powershell)* run microsoft_store command directly
- *(powershell)* remove mentions of USOClient
- *(powershell)* execution policy check breaks when run in pwsh
- *(powershell)* don't use sudo with Update-Module for pwsh
- *(powershell)* add -Command to module update cmdline
- *(tmux)* support all default `tpm` locations (xdg and both hardcoded locations) ([#1146](https://github.com/topgrade-rs/topgrade/pull/1146))
- fixed the German translation for "y/n/s/q" ([#1220](https://github.com/topgrade-rs/topgrade/pull/1220))

### Other

- *(release)* switch to release-plz ([#1333](https://github.com/topgrade-rs/topgrade/pull/1333))
- *(pre-commit)* Make pre-commit.ci use conventional commits ([#1388](https://github.com/topgrade-rs/topgrade/pull/1388))
- *(pre-commit)* pre-commit autoupdate ([#1383](https://github.com/topgrade-rs/topgrade/pull/1383))
- *(deps)* bump actions/upload-artifact from 4.6.2 to 5.0.0 ([#1382](https://github.com/topgrade-rs/topgrade/pull/1382))
- *(deps)* bump github/codeql-action from 4.30.9 to 4.31.0 ([#1379](https://github.com/topgrade-rs/topgrade/pull/1379))
- *(deps)* bump actions/download-artifact from 5.0.0 to 6.0.0 ([#1380](https://github.com/topgrade-rs/topgrade/pull/1380))
- *(deps)* bump taiki-e/install-action from 2.62.33 to 2.62.38 ([#1381](https://github.com/topgrade-rs/topgrade/pull/1381))
- *(pre-commit)* Fix pre-commit-config.yaml ([#1378](https://github.com/topgrade-rs/topgrade/pull/1378))
- *(release)* Add .deb auto completion script ([#1353](https://github.com/topgrade-rs/topgrade/pull/1353))
- *(deps)* bump github/codeql-action from 4.30.8 to 4.30.9 ([#1369](https://github.com/topgrade-rs/topgrade/pull/1369))
- *(deps)* bump taiki-e/install-action from 2.62.28 to 2.62.33 ([#1368](https://github.com/topgrade-rs/topgrade/pull/1368))
- *(deps)* bump actions/dependency-review-action from 4.8.0 to 4.8.1 ([#1362](https://github.com/topgrade-rs/topgrade/pull/1362))
- *(deps)* bump softprops/action-gh-release from 2.3.4 to 2.4.1 ([#1364](https://github.com/topgrade-rs/topgrade/pull/1364))
- *(deps)* bump taiki-e/install-action from 2.62.21 to 2.62.28 ([#1363](https://github.com/topgrade-rs/topgrade/pull/1363))
- *(deps)* bump github/codeql-action from 3.30.6 to 4.30.8 ([#1365](https://github.com/topgrade-rs/topgrade/pull/1365))
- *(deps)* bump github/codeql-action from 3.30.5 to 3.30.6 ([#1355](https://github.com/topgrade-rs/topgrade/pull/1355))
- *(deps)* bump softprops/action-gh-release from 2.3.3 to 2.3.4 ([#1356](https://github.com/topgrade-rs/topgrade/pull/1356))
- *(deps)* bump taiki-e/install-action from 2.62.13 to 2.62.21 ([#1357](https://github.com/topgrade-rs/topgrade/pull/1357))
- *(deps)* bump ossf/scorecard-action from 2.4.2 to 2.4.3 ([#1358](https://github.com/topgrade-rs/topgrade/pull/1358))
- *(deps)* bump actions/dependency-review-action from 4.7.3 to 4.8.0 ([#1350](https://github.com/topgrade-rs/topgrade/pull/1350))
- *(deps)* bump github/codeql-action from 3.30.3 to 3.30.5 ([#1349](https://github.com/topgrade-rs/topgrade/pull/1349))
- *(deps)* bump taiki-e/install-action from 2.62.1 to 2.62.13 ([#1351](https://github.com/topgrade-rs/topgrade/pull/1351))
- *(deps)* bump actions/cache from 4.2.4 to 4.3.0 ([#1352](https://github.com/topgrade-rs/topgrade/pull/1352))
- Fix WSL distribution name cleanup ([#1348](https://github.com/topgrade-rs/topgrade/pull/1348))
- *(pyproject)* mark version as dynamic ([#1347](https://github.com/topgrade-rs/topgrade/pull/1347))
- *(deps)* replace winapi with windows
- *(sudo)* rename interactive to login_shell
- Fix "WSL already reported" panic ([#1344](https://github.com/topgrade-rs/topgrade/pull/1344))
- Move step logic out of Powershell struct ([#1345](https://github.com/topgrade-rs/topgrade/pull/1345))
- *(deps)* bump taiki-e/install-action from 2.61.5 to 2.62.1 ([#1335](https://github.com/topgrade-rs/topgrade/pull/1335))
- *(deps)* bump Swatinem/rust-cache from 2.8.0 to 2.8.1 ([#1336](https://github.com/topgrade-rs/topgrade/pull/1336))
- Fixes for #1188; custom_commands broken  ([#1332](https://github.com/topgrade-rs/topgrade/pull/1332))
- use login shell when executing topgrade ([#1327](https://github.com/topgrade-rs/topgrade/pull/1327))
- *(deps)* bump taiki-e/install-action from 2.60.0 to 2.61.5 ([#1325](https://github.com/topgrade-rs/topgrade/pull/1325))
- *(deps)* bump github/codeql-action from 3.30.1 to 3.30.3 ([#1324](https://github.com/topgrade-rs/topgrade/pull/1324))
- *(pre-commit)* add typos with conservative excludes; no content changes ([#1317](https://github.com/topgrade-rs/topgrade/pull/1317))
- fix simple typos in code and comments (split var, whether, Extensions) ([#1318](https://github.com/topgrade-rs/topgrade/pull/1318))
- *(deps)* bump github/codeql-action from 3.29.11 to 3.30.1 ([#1301](https://github.com/topgrade-rs/topgrade/pull/1301))
- *(deps)* bump softprops/action-gh-release from 2.3.2 to 2.3.3 ([#1302](https://github.com/topgrade-rs/topgrade/pull/1302))
- *(deps)* bump taiki-e/install-action from 2.58.21 to 2.60.0 ([#1303](https://github.com/topgrade-rs/topgrade/pull/1303))
- *(deps)* bump actions/dependency-review-action from 4.7.2 to 4.7.3 ([#1304](https://github.com/topgrade-rs/topgrade/pull/1304))
- *(deps)* bump actions/attest-build-provenance from 2.4.0 to 3.0.0 ([#1305](https://github.com/topgrade-rs/topgrade/pull/1305))
- update tracing-subscriber to ~0.3.20 (ANSI escape injection fix, GHSA-xwfj-jgwm-7wp5) ([#1288](https://github.com/topgrade-rs/topgrade/pull/1288))
- *(deps)* bump github/codeql-action from 3.29.8 to 3.29.11 ([#1281](https://github.com/topgrade-rs/topgrade/pull/1281))
- *(deps)* bump actions/dependency-review-action from 4.7.1 to 4.7.2 ([#1282](https://github.com/topgrade-rs/topgrade/pull/1282))
- *(deps)* bump taiki-e/install-action from 2.58.9 to 2.58.21 ([#1283](https://github.com/topgrade-rs/topgrade/pull/1283))
- *(deps)* bump PyO3/maturin-action from 1.49.3 to 1.49.4 ([#1285](https://github.com/topgrade-rs/topgrade/pull/1285))
- *(deps)* bump actions/cache from 4.2.3 to 4.2.4 ([#1284](https://github.com/topgrade-rs/topgrade/pull/1284))
- Support "Insiders" versions of VSCode and VSCodium ([#1279](https://github.com/topgrade-rs/topgrade/pull/1279))
- Sudo preserve env list argument is `--preserve-env` ([#1276](https://github.com/topgrade-rs/topgrade/pull/1276))
- Clippy fixes from rust 1.91 nightly ([#1267](https://github.com/topgrade-rs/topgrade/pull/1267))
- *(deps)* bump actions/checkout from 4.2.2 to 5.0.0 ([#1264](https://github.com/topgrade-rs/topgrade/pull/1264))
- *(deps)* bump actions/download-artifact from 4.3.0 to 5.0.0 ([#1263](https://github.com/topgrade-rs/topgrade/pull/1263))
- *(deps)* bump taiki-e/install-action from 2.58.0 to 2.58.9 ([#1261](https://github.com/topgrade-rs/topgrade/pull/1261))
- *(deps)* bump ossf/scorecard-action from 2.4.0 to 2.4.2 ([#1262](https://github.com/topgrade-rs/topgrade/pull/1262))
- *(deps)* bump github/codeql-action from 3.29.5 to 3.29.8 ([#1265](https://github.com/topgrade-rs/topgrade/pull/1265))
- *(ci)* Dependabot, workflow security ([#1257](https://github.com/topgrade-rs/topgrade/pull/1257))
- replace once_cell crate with std equivalent ([#1260](https://github.com/topgrade-rs/topgrade/pull/1260))
- *(deps)* bump tokio from 1.38 to 1.47 ([#1256](https://github.com/topgrade-rs/topgrade/pull/1256))
- *(app.yml)* fix fr language #1248
- *(sudo)* add SudoKind::WinSudo
- *(sudo)* add SudoExecuteOpts builder functions and preserve_env enum
- *(yarn)* remove unnecessary Yarn::yarn field
- *(apt)* extract detect_apt() function
- route sudo usage through Sudo::execute*
- move RunType::execute to ExecutionContext
- *(powershell)* store powershell path directly
- *(powershell)* cleanup and simplify code
- Move step running into enum for dynamic ordering ([#1188](https://github.com/topgrade-rs/topgrade/pull/1188))
- Generate artifact attestations for release assets ([#1216](https://github.com/topgrade-rs/topgrade/pull/1216))
- windows update, use explicit reboot policy ([#1143](https://github.com/topgrade-rs/topgrade/pull/1143))
- add Discord invite link to README ([#1203](https://github.com/topgrade-rs/topgrade/pull/1203))
- Catch secondary uv self-update error ([#1201](https://github.com/topgrade-rs/topgrade/pull/1201))
- Handle another format change in asdf version ([#1194](https://github.com/topgrade-rs/topgrade/pull/1194))
- Preserve custom command order from config instead of sorting alphabetically ([#1182](https://github.com/topgrade-rs/topgrade/pull/1182))
- Add support for multiple binary names and idea having multiple binaries ([#1167](https://github.com/topgrade-rs/topgrade/pull/1167))
- fix the invalid action version ([#1185](https://github.com/topgrade-rs/topgrade/pull/1185))
- allow us to re-run AUR CI ([#1184](https://github.com/topgrade-rs/topgrade/pull/1184))
- Update Yazi upgrade step to use ya pkg. ([#1163](https://github.com/topgrade-rs/topgrade/pull/1163))
- use the new tag name and specify shell to bash ([#1183](https://github.com/topgrade-rs/topgrade/pull/1183))
- allow specifying tag when manually run 'create_release_assets.yml' ([#1180](https://github.com/topgrade-rs/topgrade/pull/1180))
- fix homebrew ci, remove duplicate trigger event ([#1179](https://github.com/topgrade-rs/topgrade/pull/1179))
- fix PyPI pipeline duplicate wheel name ([#1178](https://github.com/topgrade-rs/topgrade/pull/1178))
- add event workflow_dispatch to release pipelines ([#1177](https://github.com/topgrade-rs/topgrade/pull/1177))
- fix pipeline release to PyPI ([#1176](https://github.com/topgrade-rs/topgrade/pull/1176))
- Install rustfmt and clippy where necessary ([#1171](https://github.com/topgrade-rs/topgrade/pull/1171))
