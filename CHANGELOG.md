# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [16.9.0](https://github.com/topgrade-rs/topgrade/compare/v16.8.0...v16.9.0) - 2026-01-24

### Added

- *(cargo)* add `git` and `quiet` options ([#1685](https://github.com/topgrade-rs/topgrade/pull/1685))

### Other

- *(renovate)* disable rate-limiting ([#1697](https://github.com/topgrade-rs/topgrade/pull/1697))
- *(renovate)* disable rate-limiting ([#1696](https://github.com/topgrade-rs/topgrade/pull/1696))
- *(deps)* update dependency rust to v1.93.0 ([#1695](https://github.com/topgrade-rs/topgrade/pull/1695))
- *(renovate)* add rust-toolchain updating ([#1694](https://github.com/topgrade-rs/topgrade/pull/1694))
- pin toolchain to stable instead of MSRV, add MSRV testing, simplify CI workflow ([#1690](https://github.com/topgrade-rs/topgrade/pull/1690))
- *(pre-commit)* autoupdate ([#1687](https://github.com/topgrade-rs/topgrade/pull/1687))
- *(renovate)* use preset for lockfile config ([#1684](https://github.com/topgrade-rs/topgrade/pull/1684))
- *(pre-commit)* autoupdate ([#1682](https://github.com/topgrade-rs/topgrade/pull/1682))
- *(deps)* update github/codeql-action action to v4.31.10 ([#1680](https://github.com/topgrade-rs/topgrade/pull/1680))
- *(deb)* update copyright
- *(deps)* unpin toml
- *(installation)* add Pacstall Ubuntu package to README ([#1676](https://github.com/topgrade-rs/topgrade/pull/1676))
- *(deps)* update rust crate toml to v0.9.11 ([#1675](https://github.com/topgrade-rs/topgrade/pull/1675))
- *(release)* fix OpenBSD release job ([#1674](https://github.com/topgrade-rs/topgrade/pull/1674))
- *(release)* fix OpenBSD release job ([#1672](https://github.com/topgrade-rs/topgrade/pull/1672))

## [16.8.0](https://github.com/topgrade-rs/topgrade/compare/v16.7.0...v16.8.0) - 2026-01-07

### Added

- *(cursor)* add cursor extension update support ([#1659](https://github.com/topgrade-rs/topgrade/pull/1659))

### Fixed

- *(deps)* update rust crate tempfile to ~3.24.0 ([#1646](https://github.com/topgrade-rs/topgrade/pull/1646))
- *(deps)* update rust crate toml to v0.9.10 ([#1625](https://github.com/topgrade-rs/topgrade/pull/1625))

### Other

- *(deps)* update lockfile ([#1671](https://github.com/topgrade-rs/topgrade/pull/1671))
- *(release)* fix OpenBSD release job ([#1639](https://github.com/topgrade-rs/topgrade/pull/1639))
- *(pre-commit)* autoupdate ([#1668](https://github.com/topgrade-rs/topgrade/pull/1668))
- *(deps)* lock file maintenance ([#1666](https://github.com/topgrade-rs/topgrade/pull/1666))
- *(deps)* update rust crate tokio to ~1.49.0 ([#1663](https://github.com/topgrade-rs/topgrade/pull/1663))
- *(renovate)* change semantic commit type to always be 'chore' ([#1665](https://github.com/topgrade-rs/topgrade/pull/1665))
- fix category (os -> command-line-utlilities) in Cargo.toml ([#1664](https://github.com/topgrade-rs/topgrade/pull/1664))
- remove deprecated authors field from Cargo.toml ([#1661](https://github.com/topgrade-rs/topgrade/pull/1661))
- add authors and maintainers to pyproject.toml ([#1662](https://github.com/topgrade-rs/topgrade/pull/1662))
- *(deps)* update rust crate clap_complete to v4.5.64 ([#1654](https://github.com/topgrade-rs/topgrade/pull/1654))
- *(deps)* bump clap_complete ([#1657](https://github.com/topgrade-rs/topgrade/pull/1657))
- *(pre-commit)* autoupdate ([#1655](https://github.com/topgrade-rs/topgrade/pull/1655))
- *(deps)* update vmactions/openbsd-vm digest to 00753f2 ([#1649](https://github.com/topgrade-rs/topgrade/pull/1649))
- *(deps)* lock file maintenance ([#1652](https://github.com/topgrade-rs/topgrade/pull/1652))
- *(deps)* update rust crate serde_json to v1.0.148 ([#1650](https://github.com/topgrade-rs/topgrade/pull/1650))
- *(deps)* update rust crate serde_json to v1.0.147 ([#1645](https://github.com/topgrade-rs/topgrade/pull/1645))
- update bug_report issue template ([#1643](https://github.com/topgrade-rs/topgrade/pull/1643))
- *(deps)* update rust crate serde_json to v1.0.146 ([#1640](https://github.com/topgrade-rs/topgrade/pull/1640))
- add Discord link to Python package ([#1641](https://github.com/topgrade-rs/topgrade/pull/1641))
- *(deps)* pin vmactions/openbsd-vm action to a17ab0b ([#1636](https://github.com/topgrade-rs/topgrade/pull/1636))
- *(deps)* update actions/attest-build-provenance action to v3.1.0 ([#1637](https://github.com/topgrade-rs/topgrade/pull/1637))
- *(release)* add OpenBSD release steps ([#1630](https://github.com/topgrade-rs/topgrade/pull/1630))
- *(deps)* lock file maintenance ([#1631](https://github.com/topgrade-rs/topgrade/pull/1631))
- *(deps)* pin async-lock to prevent MSRV bump ([#1635](https://github.com/topgrade-rs/topgrade/pull/1635))
- *(deps)* pin async-lock to prevent MSRV bump ([#1633](https://github.com/topgrade-rs/topgrade/pull/1633))
- *(deps)* pin async-lock to prevent MSRV bump ([#1632](https://github.com/topgrade-rs/topgrade/pull/1632))
- *(deps)* update actions/attest-build-provenance action to v3.1.0 ([#1629](https://github.com/topgrade-rs/topgrade/pull/1629))
- *(deps)* update lockfile ([#1628](https://github.com/topgrade-rs/topgrade/pull/1628))
- *(deps)* update github/codeql-action action to v4.31.9 ([#1620](https://github.com/topgrade-rs/topgrade/pull/1620))
- *(release)* fix winget releases ([#1623](https://github.com/topgrade-rs/topgrade/pull/1623))
- *(release)* fix winget releases ([#1621](https://github.com/topgrade-rs/topgrade/pull/1621))

## [16.7.0](https://github.com/topgrade-rs/topgrade/compare/v16.6.1...v16.7.0) - 2025-12-17

### Added

- *(containers)* add `use_sudo` option ([#1618](https://github.com/topgrade-rs/topgrade/pull/1618))
- *(sudo)* propagate --env to sudo commands ([#1588](https://github.com/topgrade-rs/topgrade/pull/1588)) ([#1589](https://github.com/topgrade-rs/topgrade/pull/1589))
- *(aqua)* run `aqua update --config $AQUA_GLOBAL_CONFIG` instead of `aqua update` ([#1596](https://github.com/topgrade-rs/topgrade/pull/1596))

### Fixed

- *(brew)* fix brew casks and incomplete formula update on linux when using isolated user ([#1611](https://github.com/topgrade-rs/topgrade/pull/1611))
- *(vscode)* fix parsing of different version format ([#1608](https://github.com/topgrade-rs/topgrade/pull/1608))

### Other

- *(deps)* lock file maintenance ([#1615](https://github.com/topgrade-rs/topgrade/pull/1615))
- *(lint_pr)* zizmor fixes ([#1614](https://github.com/topgrade-rs/topgrade/pull/1614))
- *(renovate)* move Renovate config ([#1613](https://github.com/topgrade-rs/topgrade/pull/1613))
- *(deps)* update github/codeql-action action to v4.31.8 ([#1607](https://github.com/topgrade-rs/topgrade/pull/1607))
- *(deps)* update github artifact actions (major) ([#1609](https://github.com/topgrade-rs/topgrade/pull/1609))
- *(deps)* update rust crate shell-words to v1.1.1 ([#1604](https://github.com/topgrade-rs/topgrade/pull/1604))
- *(deps)* update itertools to 0.14.0 ([#1601](https://github.com/topgrade-rs/topgrade/pull/1601))
- *(pre-commit)* autoupdate ([#1580](https://github.com/topgrade-rs/topgrade/pull/1580))
- *(deps)* lock file maintenance ([#1597](https://github.com/topgrade-rs/topgrade/pull/1597))
- *(deps)* update github/codeql-action action to v4.31.7 ([#1591](https://github.com/topgrade-rs/topgrade/pull/1591))
- *(deps)* update release-plz/action digest to 487eb7b ([#1590](https://github.com/topgrade-rs/topgrade/pull/1590))

## [16.6.1](https://github.com/topgrade-rs/topgrade/compare/v16.6.0...v16.6.1) - 2025-12-06

### Fixed

- *(uv)* update silenced error messages ([#1593](https://github.com/topgrade-rs/topgrade/pull/1593))
- *(archlinux)* don't overwrite PATH ([#1586](https://github.com/topgrade-rs/topgrade/pull/1586))
- *(git)* skip repos with remotes without configured urls ([#1573](https://github.com/topgrade-rs/topgrade/pull/1573))
- use measure_text_width instead of byte count for header border calculation ([#1576](https://github.com/topgrade-rs/topgrade/pull/1576))

### Other

- *(deps)* update actions/checkout action to v6.0.1 ([#1583](https://github.com/topgrade-rs/topgrade/pull/1583))
- *(deps)* update actions/checkout digest to 8e8c483 ([#1582](https://github.com/topgrade-rs/topgrade/pull/1582))
- *(deps)* update dawidd6/action-homebrew-bump-formula action to v7 ([#1585](https://github.com/topgrade-rs/topgrade/pull/1585))
- *(deps)* update github/codeql-action action to v4.31.6 ([#1578](https://github.com/topgrade-rs/topgrade/pull/1578))
- *(deps)* lock file maintenance ([#1577](https://github.com/topgrade-rs/topgrade/pull/1577))
- *(deps)* update mac-notification-sys ([#1574](https://github.com/topgrade-rs/topgrade/pull/1574))

## [16.6.0](https://github.com/topgrade-rs/topgrade/compare/v16.5.0...v16.6.0) - 2025-11-28

### Added

- *(git)* add option to fetch instead of pull repositories ([#1371](https://github.com/topgrade-rs/topgrade/pull/1371))

### Fixed

- *(deps)* downgrade mac-notification-sys to fix build failure ([#1571](https://github.com/topgrade-rs/topgrade/pull/1571))
- Add more alternate names for Intellij IDEA ([#1570](https://github.com/topgrade-rs/topgrade/pull/1570))

### Other

- *(deps)* update http, tower-http, tracing, zerocopy ([#1567](https://github.com/topgrade-rs/topgrade/pull/1567))
- *(deps)* update swatinem/rust-cache action to v2.8.2 ([#1562](https://github.com/topgrade-rs/topgrade/pull/1562))
- switch release_to_pypi.yml to trusted publishing ([#1566](https://github.com/topgrade-rs/topgrade/pull/1566))
- README.md fixes ([#1564](https://github.com/topgrade-rs/topgrade/pull/1564))

## [16.5.0](https://github.com/topgrade-rs/topgrade/compare/v16.4.2...v16.5.0) - 2025-11-26

### Added

- add colors to --help/-h ([#1553](https://github.com/topgrade-rs/topgrade/pull/1553))
- *(mise)* add support for parallel job configuration in mise ([#1548](https://github.com/topgrade-rs/topgrade/pull/1548))
- *(brew)* add Homebrew cask support for Linux ([#1539](https://github.com/topgrade-rs/topgrade/pull/1539))
- *(mise)* add mise configuration options for bump and interactive modes ([#1546](https://github.com/topgrade-rs/topgrade/pull/1546))

### Fixed

- *(auto-cpufreq)* fix skipping on systems with merged bin/sbin ([#1556](https://github.com/topgrade-rs/topgrade/pull/1556))

### Other

- *(pre-commit)* autoupdate ([#1560](https://github.com/topgrade-rs/topgrade/pull/1560))
- *(deps)* update release-plz/action digest to 1efcf74 ([#1561](https://github.com/topgrade-rs/topgrade/pull/1561))
- *(deps)* lock file maintenance ([#1555](https://github.com/topgrade-rs/topgrade/pull/1555))
- *(deps)* update github/codeql-action action to v4.31.5 ([#1559](https://github.com/topgrade-rs/topgrade/pull/1559))
- *(deps)* update rust crate indexmap to v2.12.1 ([#1550](https://github.com/topgrade-rs/topgrade/pull/1550))
- *(deps)* update actions/checkout action to v6 ([#1551](https://github.com/topgrade-rs/topgrade/pull/1551))
- Add metadata to Python package for PyPI ([#1549](https://github.com/topgrade-rs/topgrade/pull/1549))
- *(installation)* update copr repo info in readme ([#1545](https://github.com/topgrade-rs/topgrade/pull/1545))

## [16.4.2](https://github.com/topgrade-rs/topgrade/compare/v16.4.1...v16.4.2) - 2025-11-20

### Other

- *(deps)* update dawidd6/action-homebrew-bump-formula action to v6 ([#1543](https://github.com/topgrade-rs/topgrade/pull/1543))

## [16.4.1](https://github.com/topgrade-rs/topgrade/compare/v16.4.0...v16.4.1) - 2025-11-20

### Other

- refactor run_containers error handling ([#1541](https://github.com/topgrade-rs/topgrade/pull/1541))

## [16.4.0](https://github.com/topgrade-rs/topgrade/compare/v16.3.0...v16.4.0) - 2025-11-20

### Added

- *(os)* add Origami Linux support ([#1530](https://github.com/topgrade-rs/topgrade/pull/1530))
- *(containers)* add option to run `system prune` ([#1523](https://github.com/topgrade-rs/topgrade/pull/1523))

### Fixed

- *(deps)* restore custom commands order ([#1535](https://github.com/topgrade-rs/topgrade/pull/1535))

### Other

- *(deps)* update clap, clap_builder, clap_complete ([#1540](https://github.com/topgrade-rs/topgrade/pull/1540))
- *(deps)* update github/codeql-action action to v4.31.4 ([#1531](https://github.com/topgrade-rs/topgrade/pull/1531))
- *(config)* add custom commands order test ([#1536](https://github.com/topgrade-rs/topgrade/pull/1536))
- make Config methods more consistent by utilizing `#[derive(Default)]` ([#1534](https://github.com/topgrade-rs/topgrade/pull/1534))
- *(issue templates)* use issue types ([#1533](https://github.com/topgrade-rs/topgrade/pull/1533))
- *(deps)* lock file maintenance ([#1505](https://github.com/topgrade-rs/topgrade/pull/1505))
- *(deps)* update actions/checkout digest to 93cb6ef ([#1526](https://github.com/topgrade-rs/topgrade/pull/1526))
- *(deps)* update actions/checkout action to v5.0.1 ([#1527](https://github.com/topgrade-rs/topgrade/pull/1527))

## [16.3.0](https://github.com/topgrade-rs/topgrade/compare/v16.2.1...v16.3.0) - 2025-11-16

### Added

- print summary and run post commands when (q)uit is used ([#1254](https://github.com/topgrade-rs/topgrade/pull/1254))
- run pre_sudo before pre_commands ([#1469](https://github.com/topgrade-rs/topgrade/pull/1469))
- *(chezmoi)* add `exclude_encrypted` config ([#1453](https://github.com/topgrade-rs/topgrade/pull/1453))

### Fixed

- *(elan)* skip running elan update on elan >=4.0.0 ([#1507](https://github.com/topgrade-rs/topgrade/pull/1507))
- *(deps)* Fix non-locked install on older version of Rust ([#1485](https://github.com/topgrade-rs/topgrade/pull/1485))
- *(deps)* Fix non-locked install on older version of Rust ([#1482](https://github.com/topgrade-rs/topgrade/pull/1482))
- *(bun)* skip self-update if not installed via official script ([#1476](https://github.com/topgrade-rs/topgrade/pull/1476))
- *(openbsd)* fix compilation on OpenBSD ([#1473](https://github.com/topgrade-rs/topgrade/pull/1473))

### Other

- *(license)* switch license variant to GPL-3.0-or-later ([#1518](https://github.com/topgrade-rs/topgrade/pull/1518))
- *(deps)* update some dependencies ([#1512](https://github.com/topgrade-rs/topgrade/pull/1512))
- *(deps)* update github/codeql-action action to v4.31.3 ([#1483](https://github.com/topgrade-rs/topgrade/pull/1483))
- remove unnecessary cfg-if dependency ([#1509](https://github.com/topgrade-rs/topgrade/pull/1509))
- *(lint_pr)* run on synchronize, and add zizmor ignore ([#1508](https://github.com/topgrade-rs/topgrade/pull/1508))
- *(pre-commit)* autoupdate ([#1464](https://github.com/topgrade-rs/topgrade/pull/1464))
- improve issue templates ([#1235](https://github.com/topgrade-rs/topgrade/pull/1235))
- *(deps)* bump mac-notification-sys, use main branch temporarily ([#1506](https://github.com/topgrade-rs/topgrade/pull/1506))
- *(deps)* lock file maintenance ([#1481](https://github.com/topgrade-rs/topgrade/pull/1481))
- *(deps)* pin dependencies ([#1478](https://github.com/topgrade-rs/topgrade/pull/1478))
- *(deps)* update actions/dependency-review-action action to v4.8.2 ([#1479](https://github.com/topgrade-rs/topgrade/pull/1479))
- Add Renovate ([#1477](https://github.com/topgrade-rs/topgrade/pull/1477))
- Replace main's self update with a proper step call ([#1470](https://github.com/topgrade-rs/topgrade/pull/1470))
- *(release)* Fix homebrew releases ([#1468](https://github.com/topgrade-rs/topgrade/pull/1468))

## [16.2.1](https://github.com/topgrade-rs/topgrade/compare/v16.2.0...v16.2.1) - 2025-11-10

### Fixed

- *(release)* Use bash in Windows to fix powershell issues ([#1461](https://github.com/topgrade-rs/topgrade/pull/1461))
- *(release)* Fix .deb distribution ([#1460](https://github.com/topgrade-rs/topgrade/pull/1460))
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
