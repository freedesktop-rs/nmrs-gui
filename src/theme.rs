use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use gpui_kit::component::theme::{Theme, ThemeConfig, ThemeMode, ThemeSet};
use gpui_kit::component::{Colorize, try_parse_color};
use gpui_kit::*;
use serde_json::{Map, Value, json};

use crate::config::Appearance;

struct Palette {
    name: &'static str,
    dark: bool,
    bg: &'static str,
    sidebar: &'static str,
    surface: &'static str,
    raised: &'static str,
    border: &'static str,
    fg: &'static str,
    muted: &'static str,
    accent: &'static str,
    on_accent: &'static str,
    danger: &'static str,
    success: &'static str,
    warning: &'static str,
}

const PALETTES: &[Palette] = &[
    Palette {
        name: "nmrs Dark",
        dark: true,
        bg: "#111113",
        sidebar: "#0c0c0e",
        surface: "#19191c",
        raised: "#222226",
        border: "#26262b",
        fg: "#ececf1",
        muted: "#8b8b96",
        accent: "#7aa2ff",
        on_accent: "#0b1020",
        danger: "#f2777a",
        success: "#6fd39b",
        warning: "#e8c06b",
    },
    Palette {
        name: "nmrs Light",
        dark: false,
        bg: "#ffffff",
        sidebar: "#f6f6f8",
        surface: "#f3f3f6",
        raised: "#eaeaef",
        border: "#e4e4ea",
        fg: "#16161a",
        muted: "#6e6e78",
        accent: "#2f6bff",
        on_accent: "#ffffff",
        danger: "#d93a3e",
        success: "#1f9d5c",
        warning: "#b77b06",
    },
    Palette {
        name: "Adwaita Dark",
        dark: true,
        bg: "#222226",
        sidebar: "#2e2e32",
        surface: "#2e2e32",
        raised: "#3a3a3f",
        border: "#3d3d42",
        fg: "#ffffff",
        muted: "#9a9996",
        accent: "#78aeed",
        on_accent: "#1e1e22",
        danger: "#ff7b63",
        success: "#8ff0a4",
        warning: "#f8e45c",
    },
    Palette {
        name: "Adwaita Light",
        dark: false,
        bg: "#fafafb",
        sidebar: "#ebebed",
        surface: "#ffffff",
        raised: "#e8e8ea",
        border: "#dcdcdf",
        fg: "#2e2e33",
        muted: "#77767b",
        accent: "#3584e4",
        on_accent: "#ffffff",
        danger: "#e01b24",
        success: "#26a269",
        warning: "#c88800",
    },
    Palette {
        name: "Catppuccin Mocha",
        dark: true,
        bg: "#1e1e2e",
        sidebar: "#181825",
        surface: "#232334",
        raised: "#313244",
        border: "#313244",
        fg: "#cdd6f4",
        muted: "#9399b2",
        accent: "#cba6f7",
        on_accent: "#1e1e2e",
        danger: "#f38ba8",
        success: "#a6e3a1",
        warning: "#f9e2af",
    },
    Palette {
        name: "Catppuccin Latte",
        dark: false,
        bg: "#eff1f5",
        sidebar: "#e6e9ef",
        surface: "#e6e9ef",
        raised: "#dce0e8",
        border: "#ccd0da",
        fg: "#4c4f69",
        muted: "#7c7f93",
        accent: "#8839ef",
        on_accent: "#eff1f5",
        danger: "#d20f39",
        success: "#40a02b",
        warning: "#df8e1d",
    },
    Palette {
        name: "Gruvbox Dark",
        dark: true,
        bg: "#282828",
        sidebar: "#1d2021",
        surface: "#32302f",
        raised: "#3c3836",
        border: "#3c3836",
        fg: "#ebdbb2",
        muted: "#a89984",
        accent: "#fabd2f",
        on_accent: "#282828",
        danger: "#fb4934",
        success: "#b8bb26",
        warning: "#fe8019",
    },
    Palette {
        name: "Gruvbox Light",
        dark: false,
        bg: "#fbf1c7",
        sidebar: "#f2e5bc",
        surface: "#f2e5bc",
        raised: "#ebdbb2",
        border: "#d5c4a1",
        fg: "#3c3836",
        muted: "#7c6f64",
        accent: "#b57614",
        on_accent: "#fbf1c7",
        danger: "#9d0006",
        success: "#79740e",
        warning: "#af3a03",
    },
    Palette {
        name: "Nord",
        dark: true,
        bg: "#2e3440",
        sidebar: "#2a2f3a",
        surface: "#353c4a",
        raised: "#3b4252",
        border: "#3b4252",
        fg: "#eceff4",
        muted: "#9aa5b8",
        accent: "#88c0d0",
        on_accent: "#2e3440",
        danger: "#bf616a",
        success: "#a3be8c",
        warning: "#ebcb8b",
    },
    Palette {
        name: "Tokyo Night",
        dark: true,
        bg: "#1a1b26",
        sidebar: "#16161e",
        surface: "#1f2335",
        raised: "#292e42",
        border: "#292e42",
        fg: "#c0caf5",
        muted: "#7a82a8",
        accent: "#7aa2f7",
        on_accent: "#1a1b26",
        danger: "#f7768e",
        success: "#9ece6a",
        warning: "#e0af68",
    },
    Palette {
        name: "Dracula",
        dark: true,
        bg: "#282a36",
        sidebar: "#21222c",
        surface: "#2f3141",
        raised: "#383a4a",
        border: "#3a3c4e",
        fg: "#f8f8f2",
        muted: "#9ea2c0",
        accent: "#bd93f9",
        on_accent: "#282a36",
        danger: "#ff5555",
        success: "#50fa7b",
        warning: "#f1fa8c",
    },
    Palette {
        name: "Rosé Pine",
        dark: true,
        bg: "#191724",
        sidebar: "#16141f",
        surface: "#1f1d2e",
        raised: "#26233a",
        border: "#26233a",
        fg: "#e0def4",
        muted: "#908caa",
        accent: "#c4a7e7",
        on_accent: "#191724",
        danger: "#eb6f92",
        success: "#9ccfd8",
        warning: "#f6c177",
    },
    Palette {
        name: "Rosé Pine Dawn",
        dark: false,
        bg: "#faf4ed",
        sidebar: "#f2e9e1",
        surface: "#fffaf3",
        raised: "#f2e9e1",
        border: "#dfdad9",
        fg: "#575279",
        muted: "#797593",
        accent: "#907aa9",
        on_accent: "#faf4ed",
        danger: "#b4637a",
        success: "#286983",
        warning: "#ea9d34",
    },
];

fn colors(p: &Palette) -> Value {
    let a = |hex: &str, alpha: &str| format!("{hex}{alpha}");
    let overlay = if p.dark { "#00000088" } else { "#00000033" };
    let pairs: Vec<(&str, String)> = vec![
        ("background", p.bg.into()),
        ("foreground", p.fg.into()),
        ("border", p.border.into()),
        ("input.border", p.border.into()),
        ("caret", p.accent.into()),
        ("ring", p.accent.into()),
        ("selection.background", a(p.accent, "55")),
        ("muted.background", p.surface.into()),
        ("muted.foreground", p.muted.into()),
        ("accent.background", p.raised.into()),
        ("accent.foreground", p.fg.into()),
        ("primary.background", p.accent.into()),
        ("primary.hover.background", a(p.accent, "e6")),
        ("primary.active.background", a(p.accent, "cc")),
        ("primary.foreground", p.on_accent.into()),
        ("secondary.background", p.surface.into()),
        ("secondary.hover.background", p.raised.into()),
        ("secondary.active.background", p.raised.into()),
        ("secondary.foreground", p.fg.into()),
        ("danger.background", p.danger.into()),
        ("danger.hover.background", a(p.danger, "e6")),
        ("danger.active.background", a(p.danger, "cc")),
        ("danger.foreground", p.bg.into()),
        ("success.background", p.success.into()),
        ("success.foreground", p.bg.into()),
        ("warning.background", p.warning.into()),
        ("warning.foreground", p.bg.into()),
        ("info.background", p.accent.into()),
        ("info.foreground", p.on_accent.into()),
        ("popover.background", p.surface.into()),
        ("popover.foreground", p.fg.into()),
        ("list.background", p.bg.into()),
        ("list.hover.background", p.surface.into()),
        ("list.active.background", a(p.accent, "1f")),
        ("list.active.border", p.accent.into()),
        ("list.even.background", p.bg.into()),
        ("list.head.background", p.bg.into()),
        ("sidebar.background", p.sidebar.into()),
        ("sidebar.foreground", p.fg.into()),
        ("sidebar.border", p.border.into()),
        ("sidebar.accent.background", p.raised.into()),
        ("sidebar.accent.foreground", p.fg.into()),
        ("sidebar.primary.background", p.accent.into()),
        ("sidebar.primary.foreground", p.on_accent.into()),
        ("switch.background", p.raised.into()),
        ("skeleton.background", p.surface.into()),
        ("scrollbar.background", "#00000000".into()),
        ("scrollbar.thumb.background", a(p.muted, "55")),
        ("scrollbar.thumb.hover.background", a(p.muted, "88")),
        ("title_bar.background", p.sidebar.into()),
        ("title_bar.border", p.border.into()),
        ("window.border", p.border.into()),
        ("overlay", overlay.into()),
        ("tab.background", "#00000000".into()),
        ("tab.active.background", p.raised.into()),
        ("tab.active.foreground", p.fg.into()),
        ("tab.foreground", p.muted.into()),
        ("tab_bar.background", p.surface.into()),
        ("tab_bar.segmented.background", p.surface.into()),
        ("group_box.background", p.surface.into()),
        ("group_box.foreground", p.fg.into()),
        ("link.foreground", p.accent.into()),
        ("link.hover.foreground", p.accent.into()),
        ("link.active.foreground", p.accent.into()),
        ("progress_bar.background", p.accent.into()),
        ("slider.bar.background", p.accent.into()),
        ("slider.thumb.background", p.fg.into()),
        ("base.red", p.danger.into()),
        ("base.green", p.success.into()),
        ("base.yellow", p.warning.into()),
        ("base.blue", p.accent.into()),
    ];
    Value::Object(
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), Value::String(v)))
            .collect(),
    )
}

pub struct Catalog {
    pub themes: Vec<Rc<ThemeConfig>>,
    pub builtin: usize,
    stamp: Vec<(PathBuf, Option<SystemTime>)>,
}

impl Global for Catalog {}

impl Catalog {
    pub fn get(&self, name: &str) -> Option<Rc<ThemeConfig>> {
        self.themes.iter().rev().find(|t| t.name == name).cloned()
    }
}

fn builtin() -> Vec<Rc<ThemeConfig>> {
    PALETTES
        .iter()
        .filter_map(|p| {
            let value = json!({
                "name": p.name,
                "mode": if p.dark { "dark" } else { "light" },
                "radius": 8,
                "radius.lg": 12,
                "shadow": false,
                "colors": colors(p),
            });
            serde_json::from_value::<ThemeConfig>(value)
                .ok()
                .map(Rc::new)
        })
        .collect()
}

fn scan(dir: &Path) -> Vec<(PathBuf, Option<SystemTime>)> {
    let mut entries: Vec<(PathBuf, Option<SystemTime>)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .map(|p| {
            let mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            (p, mtime)
        })
        .collect();
    entries.sort();
    entries
}

fn load_user(stamp: &[(PathBuf, Option<SystemTime>)]) -> (Vec<Rc<ThemeConfig>>, Vec<String>) {
    let mut themes = Vec::new();
    let mut errors = Vec::new();
    for (path, _) in stamp {
        let parsed = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str::<ThemeSet>(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(set) => themes.extend(set.themes.into_iter().map(Rc::new)),
            Err(err) => errors.push(format!(
                "{}: {err}",
                path.file_name().unwrap_or_default().to_string_lossy()
            )),
        }
    }
    (themes, errors)
}

pub fn load(cx: &mut App) -> Vec<String> {
    let stamp = scan(&crate::config::themes_dir());
    let mut themes = builtin();
    let builtin = themes.len();
    let (user, errors) = load_user(&stamp);
    themes.extend(user);
    cx.set_global(Catalog {
        themes,
        builtin,
        stamp,
    });
    errors
}

pub fn reload_if_changed(cx: &mut App) -> Option<Vec<String>> {
    let stamp = scan(&crate::config::themes_dir());
    if cx.global::<Catalog>().stamp == stamp {
        return None;
    }
    Some(load(cx))
}

pub fn names(cx: &App) -> Vec<(SharedString, bool)> {
    let mut seen = std::collections::HashSet::new();
    cx.global::<Catalog>()
        .themes
        .iter()
        .filter(|t| seen.insert(t.name.clone()))
        .map(|t| (t.name.clone(), t.mode.is_dark()))
        .collect()
}

pub fn swatch(name: &str, cx: &App) -> Option<[Hsla; 4]> {
    let theme = cx.global::<Catalog>().get(name)?;
    let value = serde_json::to_value(&theme.colors).ok()?;
    let get = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .and_then(|s| try_parse_color(s).ok())
    };
    Some([
        get("background")?,
        get("sidebar.background").or(get("background"))?,
        get("foreground")?,
        get("primary.background")?,
    ])
}

pub fn resolve_name(appearance: &Appearance, dark: bool) -> String {
    if appearance.theme == "system" {
        if dark {
            appearance.dark_theme.clone()
        } else {
            appearance.light_theme.clone()
        }
    } else {
        appearance.theme.clone()
    }
}

pub fn apply(appearance: &Appearance, window: Option<&mut Window>, cx: &mut App) {
    let dark = window
        .as_ref()
        .map(|w| {
            matches!(
                w.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            )
        })
        .unwrap_or(true);
    let name = resolve_name(appearance, dark);
    let catalog = cx.global::<Catalog>();
    let fallback = if dark { "nmrs Dark" } else { "nmrs Light" };
    let Some(config) = catalog.get(&name).or_else(|| catalog.get(fallback)) else {
        return;
    };

    let config = with_overrides(&config, appearance);
    let mode = config.mode;

    let theme = Theme::global_mut(cx);
    theme.apply_config(&config);
    theme.mode = mode;
    theme.font_size = px(appearance.font_size.clamp(10.0, 24.0));
    theme.radius = px(appearance.radius.clamp(0.0, 24.0));
    theme.radius_lg = px((appearance.radius * 1.5).clamp(0.0, 32.0));
    if let Some(family) = appearance.font_family.as_ref().filter(|f| !f.is_empty()) {
        theme.font_family = family.clone().into();
    }
    let opacity = appearance.opacity.clamp(0.0, 1.0);
    if opacity < 1.0 {
        theme.colors.background = theme.colors.background.opacity(opacity);
        theme.colors.sidebar = theme.colors.sidebar.opacity(opacity);
        theme.colors.title_bar = theme.colors.title_bar.opacity(opacity);
        theme.colors.list = theme.colors.list.opacity(0.0);
    }
    cx.refresh_windows();
}

fn with_overrides(config: &Rc<ThemeConfig>, appearance: &Appearance) -> Rc<ThemeConfig> {
    let Some(accent) = appearance
        .accent
        .as_ref()
        .filter(|a| try_parse_color(a).is_ok())
    else {
        return config.clone();
    };
    let Ok(mut value) = serde_json::to_value(config.as_ref()) else {
        return config.clone();
    };
    if let Some(colors) = value.get_mut("colors").and_then(Value::as_object_mut) {
        let set = |colors: &mut Map<String, Value>, key: &str, v: String| {
            colors.insert(key.to_string(), Value::String(v));
        };
        let hex = accent.trim_start_matches('#');
        let base = format!("#{}", &hex[..hex.len().min(6)]);
        for key in [
            "primary.background",
            "ring",
            "caret",
            "list.active.border",
            "sidebar.primary.background",
            "link.foreground",
            "link.hover.foreground",
            "link.active.foreground",
            "progress_bar.background",
            "slider.bar.background",
            "info.background",
            "base.blue",
        ] {
            set(colors, key, base.clone());
        }
        set(colors, "primary.hover.background", format!("{base}e6"));
        set(colors, "primary.active.background", format!("{base}cc"));
        set(colors, "selection.background", format!("{base}55"));
        set(colors, "list.active.background", format!("{base}1f"));
        let dark = config.mode.is_dark();
        let on = try_parse_color(&base)
            .map(|c| if c.l > 0.6 { "#111113" } else { "#ffffff" })
            .unwrap_or(if dark { "#111113" } else { "#ffffff" });
        set(colors, "primary.foreground", on.to_string());
        set(colors, "sidebar.primary.foreground", on.to_string());
    }
    serde_json::from_value(value)
        .map(Rc::new)
        .unwrap_or_else(|_| config.clone())
}

pub fn is_dark(cx: &App) -> bool {
    Theme::global(cx).mode == ThemeMode::Dark
}
