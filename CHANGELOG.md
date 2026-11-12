# Changelog

All notable changes to the `nmrs-gui` crate will be documented in this file.

## [Unreleased]
### Added
- `--status` reports tunnels not managed by NetworkManager (ex. OpenVPN DCO `tun0`) as `"managed": false`. The VPN list shows them read-only ([#87](https://github.com/freedesktop-rs/nmrs-gui/pull/87))
- `--status` prints the network state as JSON, and `--status --watch` streams a line on every change, for bars and widgets ([#75](https://github.com/freedesktop-rs/nmrs-gui/pull/75))
- `nmrs-bin` AUR package, prebuilt from the release binary and published automatically on release ([#80](https://github.com/freedesktop-rs/nmrs-gui/pull/80))
- A control socket at `$XDG_RUNTIME_DIR/nmrs/<app-id>.sock`: `--toggle`, `--show`, `--hide` and `--quit` drive the running instance, a plain relaunch raises it, and a hidden instance stays resident so the next toggle is instant ([#74](https://github.com/freedesktop-rs/nmrs-gui/pull/74))

## [2.1.0] - 2026-11-03
### Added
- Keyboard navigation: `Tab` reaches every sidebar item and list row, `↑`/`↓` move between rows, and `Enter` activates the focused one ([#67](https://github.com/freedesktop-rs/nmrs-gui/pull/67))

## [2.0.1] - 2026-11-02
### Changed
- Release builds use thin LTO, cutting peak build memory from 14+ GB to about 1.9 GB ([#63](https://github.com/freedesktop-rs/nmrs-gui/pull/63))
- Minimum supported Rust version is now 1.95 ([#59](https://github.com/freedesktop-rs/nmrs-gui/pull/59))
- Update gpui-kit to 0.9.0 ([#58](https://github.com/freedesktop-rs/nmrs-gui/pull/58)), toml to 1.1.8 ([#56](https://github.com/freedesktop-rs/nmrs-gui/pull/56), [#57](https://github.com/freedesktop-rs/nmrs-gui/pull/57)) and serde_json to 1.0.154 ([#55](https://github.com/freedesktop-rs/nmrs-gui/pull/55))
- CI skips the release build, caches from master only, and runs formatting in its own job ([#61](https://github.com/freedesktop-rs/nmrs-gui/pull/61))

### Fixed
- AUR builds being killed by the OOM killer on 16 GB machines ([#63](https://github.com/freedesktop-rs/nmrs-gui/pull/63))
- Nix builds failing on Rust 1.94 and using deprecated `xorg.*` aliases ([#59](https://github.com/freedesktop-rs/nmrs-gui/pull/59))
- `package.nix` reads crate hashes from `Cargo.lock`, removing the `cargoHash` fixer workflow ([#62](https://github.com/freedesktop-rs/nmrs-gui/pull/62))

## [2.0.0] - 2026-11-01
### Changed
- Rewrote the interface on gpui, replacing GTK4 ([#52](https://github.com/freedesktop-rs/nmrs-gui/pull/52))

### Added
- Wi-Fi: WPA3, Enhanced Open and Enterprise (PEAP, TTLS, TLS), BSSID pinning, hidden networks, hotspot ([#52](https://github.com/freedesktop-rs/nmrs-gui/pull/52))
- Profiles page with an IPv4/IPv6 editor and raw settings view ([#52](https://github.com/freedesktop-rs/nmrs-gui/pull/52))
- Network page with airplane mode, radios, connectivity checks, captive portal sign-in, global DNS and interface management ([#52](https://github.com/freedesktop-rs/nmrs-gui/pull/52))
- Bluetooth tethering, OpenVPN import and per-tunnel autoconnect ([#52](https://github.com/freedesktop-rs/nmrs-gui/pull/52))
- `config.toml` with live reload, built-in and user JSON themes, compact layout, density, opacity and floating window options ([#52](https://github.com/freedesktop-rs/nmrs-gui/pull/52))
- `--page`, `--layout`, `--app-id`, `--config` and `--demo` flags ([#52](https://github.com/freedesktop-rs/nmrs-gui/pull/52))
- Option to follow the system color scheme ([#39](https://github.com/freedesktop-rs/nmrs-gui/pull/39))

### Fixed
- Systems without Wi-Fi hardware ([#38](https://github.com/freedesktop-rs/nmrs-gui/pull/38))

## [1.6.0] - 2026-05-20
### Added
- VPN management: list, connect, disconnect, and add WireGuard and OpenVPN profiles ([#21](https://github.com/networkmanager-rs/nmrs-gui/pull/21))

### Changed
- Header shows a connection-type icon and active network name (VPN, wired, Wi‑Fi, or disconnected) ([#21](https://github.com/networkmanager-rs/nmrs-gui/pull/21))
- Network scan progress uses a header spinner instead of "Scanning..." text ([#21](https://github.com/networkmanager-rs/nmrs-gui/pull/21))
- Use `Arc<Notify>` for monitor callbacks to satisfy `Send` bound ([#359](https://github.com/cachebag/nmrs/pull/359))

## [1.5.1] - 2026-04-10
### Fixed
- Set a location for our `.lock` file ([#310](https://github.com/cachebag/nmrs/pull/310))

## [1.5.0] - 2026-03-19
### Fixed
- Custom `~/.config/nmrs/style.css` is now applied after the saved theme, ensuring it always takes precedence over any predefined theme ([#274](https://github.com/cachebag/nmrs/pull/274))
- Fixed header status label inflating natural width with long status strings ([#279](https://github.com/cachebag/nmrs/pull/279))

## [1.1.0] - 2025-12-19

### Fixed
- Corrected binary name for `.desktop` file + `postInstall` hook for Nix flake ([#146](https://github.com/cachebag/nmrs/pull/146))

## [0.5.0-beta] - 2025-12-15

### Added
- Full support for Ethernet devices ([#88](https://github.com/cachebag/nmrs/issues/88))

### Fixed
- Fixed UI freeze when connecting/forgetting networks
- Supply option to provide cert paths for WPA-EAP connections ([#56](https://github.com/cachebag/nmrs/issues/56))

## [0.4.0-beta] - 2025-12-11

### Breaking Changes
- Renamed crate from `nmrs-ui` to `nmrs-gui`

### Added
- Pre-defined themes (Catppuccin, Dracula, Gruvbox, Nord, Tokyo) ([#106](https://github.com/cachebag/nmrs/issues/106))
- `--version` flag with build hash extraction ([#108](https://github.com/cachebag/nmrs/issues/108))

### Fixed
- Re-aligned refresh button ([#111](https://github.com/cachebag/nmrs/issues/111))
- Show connection status when connecting with saved credentials ([#61](https://github.com/cachebag/nmrs/issues/61))

## [0.3.0-beta] - 2025-12-08

### Fixed
- Fixed UI not freezing on connections ([#101](https://github.com/cachebag/nmrs/pull/101))
- Fixed separate `ScrolledWindow` for each stack child ([#103](https://github.com/cachebag/nmrs/pull/103))
- Dropped deps that aren't needed for now ([#104](https://github.com/cachebag/nmrs/pull/104))

### Added
- Expose system default theme toggle (light/dark) ([#102](https://github.com/cachebag/nmrs/pull/102))

## [0.2.0-beta] - 2025-12-03

### Added
- Write `.css` file for user by default ([#58](https://github.com/cachebag/nmrs/pull/58))
- Config: Nix installation deps ([#60](https://github.com/cachebag/nmrs/pull/60))
- Visual indication on successful connection ([#64](https://github.com/cachebag/nmrs/pull/64))
- Refactored `network.rs` and `network_page.rs` to follow best practices ([#66](https://github.com/cachebag/nmrs/pull/90))

## [0.1.1-beta] - 2025-11-21

### Added
- Exposed layout to allow users to place a `style.css` in `~/.config/nmrs/` for custom styling ([#55](https://github.com/cachebag/nmrs/pull/55))
- Styling section with example CSS for customization in README

## [0.1.0-beta] - 2025-11-20

### Added
- Initial BETA release of nmrs GUI
- GTK4-based user interface
- Basic and advanced network information pages
- Persistent saved-connection state tracking
- Refresh button for manual network scanning ([#51](https://github.com/cachebag/nmrs/pull/51))
- `.desktop` file for launcher integration
- AUR package support (via `yay` and `paru`)
- Initial smoke tests ([#48](https://github.com/cachebag/nmrs/pull/48))
- Issue templates for bug reports and feature requests
- Contributing guidelines (CONTRIBUTING.md)
- Installation support for Arch Linux, Debian/Ubuntu, and from source

### Fixed
- UI enhancements for connected devices
- Password authentication failure handling
- `refresh_networks` helper for proper scanning ([#50](https://github.com/cachebag/nmrs/pull/50))
- Adjusted polling to accurately display networks on launch
- `forget_btn` thread locks
- Context correction for network settings

### Documentation
- Initial README with installation and usage instructions

[1.5.0]: https://github.com/networkmanager-rs/nmrs-gui/compare/v1.1.0...v1.5.0
[1.5.1]: https://github.com/networkmanager-rs/nmrs-gui/compare/v1.5.0...v1.5.1
[Unreleased]: https://github.com/freedesktop-rs/nmrs-gui/compare/v2.1.0...HEAD
[2.1.0]: https://github.com/freedesktop-rs/nmrs-gui/compare/v2.0.1...v2.1.0
[2.0.1]: https://github.com/freedesktop-rs/nmrs-gui/compare/v2.0.0...v2.0.1
[2.0.0]: https://github.com/freedesktop-rs/nmrs-gui/compare/nmrs-gui-v1.6.0...v2.0.0
[1.6.0]: https://github.com/networkmanager-rs/nmrs-gui/compare/v1.5.1...nmrs-gui-v1.6.0
[1.1.0]: https://github.com/networkmanager-rs/nmrs-gui/compare/v0.5.0-beta...v1.1.0
[0.5.0-beta]: https://github.com/networkmanager-rs/nmrs-gui/compare/v0.4.0-beta...v0.5.0-beta
[0.4.0-beta]: https://github.com/networkmanager-rs/nmrs-gui/compare/v0.3.0-beta...v0.4.0-beta
[0.3.0-beta]: https://github.com/networkmanager-rs/nmrs-gui/compare/v0.2.0-beta...v0.3.0-beta
[0.2.0-beta]: https://github.com/networkmanager-rs/nmrs-gui/compare/v0.1.1-beta...v0.2.0-beta
[0.1.1-beta]: https://github.com/networkmanager-rs/nmrs-gui/compare/v0.1.0-beta...v0.1.1-beta
[0.1.0-beta]: https://github.com/networkmanager-rs/nmrs-gui/releases/tag/v0.1.0-beta

