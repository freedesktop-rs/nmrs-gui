# <p align="center"> nmrs 🦀

A GTK4 GUI for managing NetworkManager connections on Linux. Built with Rust and libadwaita.

[![CI](https://github.com/networkmanager-rs/nmrs-gui/actions/workflows/ci.yml/badge.svg)](https://github.com/networkmanager-rs/nmrs-gui/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/nmrs-gui)](https://crates.io/crates/nmrs-gui)

<p align="center">
  <img src="https://github.com/user-attachments/assets/472308a0-f902-41a1-a6d0-66bffb7793f8" width="300" height="300" />

  <img src="https://github.com/user-attachments/assets/d495e4a1-d505-4e0a-80ce-edaf8a67b739" width="300" height="300" />

  <img src="https://github.com/user-attachments/assets/84d6ca5e-4e65-4d91-ae59-c86d5e1d825a" width="300" height="300" />
</p>

## Features

- Full VPN support for OpenVPN and WireGuard
- Connect to, disconnect from, and forget Wi-Fi networks
- Full Ethernet device support
- WPA-Enterprise (EAP) connections with certificate path support
- Pre-defined themes: Catppuccin, Dracula, Gruvbox, Nord, Tokyo Night
- Custom CSS theming via `~/.config/nmrs/style.css`
- System light/dark mode toggle

## Installation

### Arch Linux (AUR)

```bash
yay -S nmrs
# or
paru -S nmrs
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

Both install the `nmrs` binary.

## Usage

```bash
nmrs [OPTIONS]

Options:
  -V, --version    Print version and build hash
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

## License

MIT — see [LICENSE-MIT](LICENSE-MIT).
