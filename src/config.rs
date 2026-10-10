use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui_kit::assets::IconName;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub appearance: Appearance,
    pub layout: Layout,
    pub wifi: Wifi,
    pub devices: Devices,
    pub behavior: Behavior,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Devices {
    pub show_virtual: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub theme: String,
    pub light_theme: String,
    pub dark_theme: String,
    pub accent: Option<String>,
    pub font_family: Option<String>,
    pub font_size: f32,
    pub radius: f32,
    pub opacity: f32,
    pub density: Density,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            light_theme: "nmrs Light".into(),
            dark_theme: "nmrs Dark".into(),
            accent: None,
            font_family: None,
            font_size: 14.0,
            radius: 8.0,
            opacity: 1.0,
            density: Density::Comfortable,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

impl Density {
    pub const ALL: [Density; 3] = [Self::Compact, Self::Comfortable, Self::Spacious];

    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Comfortable => "Comfortable",
            Self::Spacious => "Spacious",
        }
    }

    pub fn row_height(self) -> f32 {
        match self {
            Self::Compact => 36.0,
            Self::Comfortable => 46.0,
            Self::Spacious => 56.0,
        }
    }

    pub fn gap(self) -> f32 {
        match self {
            Self::Compact => 12.0,
            Self::Comfortable => 20.0,
            Self::Spacious => 28.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub mode: LayoutMode,
    pub pages: Vec<Page>,
    pub width: f32,
    pub height: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            mode: LayoutMode::Sidebar,
            pages: Page::ALL.to_vec(),
            width: 920.0,
            height: 620.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    #[default]
    Sidebar,
    Compact,
}

impl LayoutMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sidebar => "Sidebar",
            Self::Compact => "Compact",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Page {
    Wifi,
    Ethernet,
    Vpn,
    Bluetooth,
    Profiles,
    Network,
    Appearance,
}

impl Page {
    pub const ALL: [Page; 7] = [
        Self::Wifi,
        Self::Ethernet,
        Self::Vpn,
        Self::Bluetooth,
        Self::Profiles,
        Self::Network,
        Self::Appearance,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Wifi => "Wi-Fi",
            Self::Ethernet => "Ethernet",
            Self::Vpn => "VPN",
            Self::Bluetooth => "Bluetooth",
            Self::Profiles => "Profiles",
            Self::Network => "Network",
            Self::Appearance => "Appearance",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Wifi => IconName::Wifi,
            Self::Ethernet => IconName::EthernetPort,
            Self::Vpn => IconName::Shield,
            Self::Bluetooth => IconName::Bluetooth,
            Self::Profiles => IconName::Bookmark,
            Self::Network => IconName::Globe,
            Self::Appearance => IconName::Palette,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Wifi {
    pub signal: SignalStyle,
    pub show_band: bool,
    pub show_bssid: bool,
    pub show_security: bool,
    pub min_strength: u8,
    pub sort: Sort,
    pub scan_on_open: bool,
}

impl Default for Wifi {
    fn default() -> Self {
        Self {
            signal: SignalStyle::Icon,
            show_band: true,
            show_bssid: false,
            show_security: true,
            min_strength: 0,
            sort: Sort::Signal,
            scan_on_open: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalStyle {
    #[default]
    Icon,
    Percent,
    Both,
}

impl SignalStyle {
    pub const ALL: [SignalStyle; 3] = [Self::Icon, Self::Percent, Self::Both];

    pub fn label(self) -> &'static str {
        match self {
            Self::Icon => "Icon",
            Self::Percent => "Percent",
            Self::Both => "Both",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    #[default]
    Signal,
    Name,
}

impl Sort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Signal => "Signal",
            Self::Name => "Name",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Behavior {
    pub close_on_connect: bool,
    pub close_on_escape: bool,
    pub confirm_forget: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            close_on_connect: false,
            close_on_escape: true,
            confirm_forget: true,
        }
    }
}

pub fn dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nmrs")
}

pub fn themes_dir() -> PathBuf {
    dir().join("themes")
}

pub struct Store {
    path: PathBuf,
    modified: Option<SystemTime>,
}

impl Store {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            path: path.unwrap_or_else(|| dir().join("config.toml")),
            modified: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn mtime(&self) -> Option<SystemTime> {
        std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok()
    }

    pub fn load(&mut self) -> Result<Config, String> {
        self.modified = self.mtime();
        match std::fs::read_to_string(&self.path) {
            Ok(text) => toml::from_str(&text).map_err(|e| e.message().to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn changed(&self) -> bool {
        self.mtime() != self.modified
    }

    pub fn save(&mut self, config: &Config) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, text).map_err(|e| e.to_string())?;
        self.modified = self.mtime();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_yields_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn partial_sections_keep_other_defaults() {
        let config: Config = toml::from_str(
            r##"
            [appearance]
            theme = "Nord"
            accent = "#88c0d0"

            [layout]
            mode = "compact"
            pages = ["wifi", "vpn"]
            "##,
        )
        .unwrap();
        assert_eq!(config.appearance.theme, "Nord");
        assert_eq!(config.appearance.accent.as_deref(), Some("#88c0d0"));
        assert_eq!(config.appearance.font_size, 14.0);
        assert_eq!(config.layout.mode, LayoutMode::Compact);
        assert_eq!(config.layout.pages, vec![Page::Wifi, Page::Vpn]);
        assert_eq!(config.wifi, Wifi::default());
    }

    #[test]
    fn round_trips_through_toml() {
        let mut config = Config::default();
        config.wifi.signal = SignalStyle::Both;
        config.appearance.density = Density::Spacious;
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }

    #[test]
    fn store_saves_and_detects_external_edits() {
        let dir = std::env::temp_dir().join(format!("nmrs-gui-test-{}", std::process::id()));
        let path = dir.join("config.toml");
        let mut store = Store::new(Some(path.clone()));
        assert_eq!(store.load().unwrap(), Config::default());

        let mut config = Config::default();
        config.behavior.close_on_connect = true;
        store.save(&config).unwrap();
        assert!(!store.changed());
        assert_eq!(store.load().unwrap(), config);

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "[behavior]\nclose_on_escape = false\n").unwrap();
        assert!(store.changed());
        assert!(!store.load().unwrap().behavior.close_on_escape);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn invalid_file_reports_error() {
        let dir = std::env::temp_dir().join(format!("nmrs-gui-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "[layout]\nmode = \"diagonal\"\n").unwrap();
        assert!(Store::new(Some(path)).load().is_err());
        std::fs::remove_dir_all(dir).ok();
    }
}
