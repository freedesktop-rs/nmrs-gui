# contrib

Bar integrations that consume `nmrs-gui --status --watch` and open the window with `nmrs-gui --toggle`.

## Quickshell

Copy `quickshell/Nmrs.qml` next to your shell and place it like any other item:

```qml
Nmrs {
    appId: "nmrs-popup"
    font: "JetBrainsMono Nerd Font"
}
```

Left click toggles the compact window, right click toggles it on the VPN page. Colors follow the theme nmrs-gui resolves from `config.toml`, so the bar item matches the window. `toggleCommand` and `rightClickCommand` are plain string lists if you want different arguments.

## waybar

`waybar/nmrs-status` turns the stream into waybar's JSON. Add the module from `waybar/config.jsonc` and put `custom/nmrs` where `network` was:

```css
#custom-nmrs.offline { color: #888888; }
#custom-nmrs.portal { color: #f7768e; }
#custom-nmrs.vpn { color: #9ece6a; }
```

Both need a Nerd Font for the glyphs.
