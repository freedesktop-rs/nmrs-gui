# <p align="center"> nmrs-gui 🦀 

A GTK4 GUI for managing NetworkManager connections on Linux. Built with Rust and libadwaita.

[![CI](https://github.com/networkmanager-rs/nmrs-gui/actions/workflows/ci.yml/badge.svg)](https://github.com/networkmanager-rs/nmrs-gui/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/nmrs-gui)](https://crates.io/crates/nmrs-gui)

## Features

- Connect to, disconnect from, and forget Wi-Fi networks
- Full Ethernet device support
- WPA-Enterprise (EAP) connections with certificate path support
- Pre-defined themes: Catppuccin, Dracula, Gruvbox, Nord, Tokyo Night
- Custom CSS theming via `~/.config/nmrs/style.css`
- System light/dark mode toggle
- Waybar-compatible status output
- Single-instance enforcement via file lock

## Installation

### Arch Linux (AUR)

```bash
yay -S nmrs-gui
# or
paru -S nmrs-gui
```

### From crates.io

```bash
cargo install nmrs-gui
```

### From source

```bash
# Install GTK4 + libadwaita first (see CONTRIBUTING.md for full dep list)
cargo install --path .
```

### Nix / NixOS

```bash
nix run github:networkmanager-rs/nmrs-gui
```

Or add to your flake inputs and use `packages.${system}.default`.

## Usage

```bash
nmrs-gui [OPTIONS]

Options:
  -v, --version    Print version and build hash
  -h, --help       Print help
```

## Theming

Place a `style.css` in `~/.config/nmrs/` to apply custom styles on top of any
pre-defined theme. Your overrides are always loaded last, so they take
precedence.

```css
/* ~/.config/nmrs/style.css */
window {
  background-color: #1e1e2e;
}
```

Choose a built-in theme from the Settings page inside the app.

## Waybar Integration

```json
{
  "custom/nmrs": {
    "exec": "nmrs-gui --status",
    "interval": 5,
    "format": "  {}"
  }
}
```

## License

MIT — see [LICENSE-MIT](LICENSE-MIT).
