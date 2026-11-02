# Contributing to nmrs-gui

Thanks for your interest in contributing!

## Prerequisites

nmrs-gui is built on gpui and needs a few system libraries:

**Arch Linux**

```bash
sudo pacman -S pkgconf fontconfig freetype2 libxkbcommon libxkbcommon-x11 wayland vulkan-icd-loader
```

**Debian / Ubuntu**

```bash
sudo apt-get install -y \
  pkg-config \
  libfontconfig-dev \
  libfreetype-dev \
  libwayland-dev \
  libxkbcommon-dev \
  libxkbcommon-x11-dev \
  libx11-xcb-dev \
  libvulkan-dev
```

**Fedora**

```bash
sudo dnf install pkgconf fontconfig-devel freetype-devel libxkbcommon-devel libxkbcommon-x11-devel wayland-devel vulkan-loader-devel
```

**Nix**

```bash
nix develop
```

## Building

```bash
cargo build
cargo test
cargo run -- --demo
```

`--demo` runs the interface against sample data, so UI work never touches your live connections.

## Code Style

- `cargo fmt` before committing
- `cargo clippy -- -D warnings` must pass
- Keep commits focused; reference the relevant issue in the message

## Releases

Releases are cut by pushing a `v*` tag (e.g. `v1.6.0`). The release workflow
builds the binary and publishes a GitHub release automatically. Update
`CHANGELOG.md` and bump the version in `Cargo.toml` before tagging.
