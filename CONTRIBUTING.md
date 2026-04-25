# Contributing to nmrs-gui

Thanks for your interest in contributing!

## Prerequisites

nmrs-gui requires GTK4 and libadwaita. Install them before building:

**Arch Linux**

```bash
sudo pacman -S gtk4 libadwaita pkg-config
```

**Debian / Ubuntu**

```bash
sudo apt-get install -y \
  pkg-config \
  libglib2.0-dev \
  libgirepository1.0-dev \
  libgdk-pixbuf2.0-dev \
  libpango1.0-dev \
  libcairo2-dev \
  libgtk-4-dev \
  libadwaita-1-dev
```

**Fedora**

```bash
sudo dnf install gtk4-devel libadwaita-devel pkg-config
```

**Nix**

```bash
nix develop
```

## Building

```bash
cargo build
cargo test
```

## Code Style

- `cargo fmt` before committing
- `cargo clippy -- -D warnings` must pass
- Keep commits focused; reference the relevant issue in the message

## Releases

Releases are cut by pushing a `v*` tag (e.g. `v1.6.0`). The release workflow
builds the binary and publishes a GitHub release automatically. Update
`CHANGELOG.md` and bump the version in `Cargo.toml` before tagging.
