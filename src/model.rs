#[derive(Clone, Debug, Default)]
pub struct State {
    pub radios: Radios,
    pub airplane: bool,
    pub connectivity: Connectivity,
    pub wifi_devices: Vec<WifiDevice>,
    pub networks: Vec<WifiNetwork>,
    pub wired: Vec<WiredDevice>,
    pub vpns: Vec<Vpn>,
    pub bluetooth: Vec<BluetoothDevice>,
    pub profiles: Vec<Profile>,
    pub dns: GlobalDns,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Radio {
    pub enabled: bool,
    pub hardware: bool,
    pub present: bool,
}

impl Radio {
    pub fn usable(&self) -> bool {
        self.present && self.hardware
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Radios {
    pub wifi: Radio,
    pub wwan: Radio,
    pub bluetooth: Radio,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reachability {
    #[default]
    Unknown,
    None,
    Portal,
    Limited,
    Full,
}

impl Reachability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::None => "Offline",
            Self::Portal => "Sign-in required",
            Self::Limited => "Limited",
            Self::Full => "Online",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Connectivity {
    pub state: Reachability,
    pub checking: bool,
    pub uri: Option<String>,
    pub portal: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinkState {
    Unmanaged,
    Unavailable,
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Failed,
}

impl LinkState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unmanaged => "Unmanaged",
            Self::Unavailable => "Unavailable",
            Self::Disconnected => "Disconnected",
            Self::Connecting => "Connecting",
            Self::Connected => "Connected",
            Self::Disconnecting => "Disconnecting",
            Self::Failed => "Failed",
        }
    }

    pub fn is_up(self) -> bool {
        matches!(self, Self::Connected)
    }

    pub fn is_busy(self) -> bool {
        matches!(self, Self::Connecting | Self::Disconnecting)
    }
}

#[derive(Clone, Debug, Default)]
pub struct WifiDevice {
    pub interface: String,
    pub mac: String,
    pub driver: Option<String>,
    pub state: LinkState,
    pub active_ssid: Option<String>,
    pub managed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Security {
    #[default]
    Open,
    Owe,
    Wep,
    Psk,
    Sae,
    Eap,
}

impl Security {
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Owe => "Enhanced Open",
            Self::Wep => "WEP",
            Self::Psk => "WPA2",
            Self::Sae => "WPA3",
            Self::Eap => "Enterprise",
        }
    }

    pub fn needs_secret(self) -> bool {
        matches!(self, Self::Wep | Self::Psk | Self::Sae)
    }

    pub fn is_secured(self) -> bool {
        !matches!(self, Self::Open | Self::Owe)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AccessPoint {
    pub bssid: String,
    pub strength: u8,
    pub frequency: u32,
    pub rate_mbps: u32,
    pub active: bool,
}

#[derive(Clone, Debug, Default)]
pub struct WifiNetwork {
    pub ssid: String,
    pub interface: String,
    pub strength: u8,
    pub frequency: u32,
    pub security: Security,
    pub active: bool,
    pub known: bool,
    pub profile: Option<String>,
    pub access_points: Vec<AccessPoint>,
    pub ip4: Option<String>,
    pub ip6: Option<String>,
}

impl WifiNetwork {
    pub fn band(&self) -> Option<&'static str> {
        band(self.frequency)
    }

    pub fn channel(&self) -> Option<u32> {
        channel(self.frequency)
    }
}

pub fn band(frequency: u32) -> Option<&'static str> {
    match frequency {
        2400..=2500 => Some("2.4 GHz"),
        4900..=5900 => Some("5 GHz"),
        5925..=7125 => Some("6 GHz"),
        _ => None,
    }
}

pub fn channel(frequency: u32) -> Option<u32> {
    match frequency {
        2484 => Some(14),
        2412..=2472 => Some((frequency - 2407) / 5),
        5000..=5900 => Some((frequency - 5000) / 5),
        5955..=7115 => Some((frequency - 5950) / 5),
        _ => None,
    }
}

#[derive(Clone, Debug, Default)]
pub struct WiredDevice {
    pub interface: String,
    pub mac: String,
    pub driver: Option<String>,
    pub speed_mbps: Option<u32>,
    pub state: LinkState,
    pub profile: Option<String>,
    pub profile_uuid: Option<String>,
    pub ip4: Option<String>,
    pub ip6: Option<String>,
    pub managed: bool,
    pub autoconnect: bool,
}

impl WiredDevice {
    pub fn is_virtual(&self) -> bool {
        matches!(
            self.driver.as_deref(),
            Some("veth" | "bridge" | "tun" | "dummy" | "macvlan" | "vxlan")
        ) || self.interface.starts_with("veth")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VpnKind {
    WireGuard,
    OpenVpn,
    OpenConnect,
    StrongSwan,
    Pptp,
    L2tp,
    Other(String),
}

impl VpnKind {
    pub fn label(&self) -> &str {
        match self {
            Self::WireGuard => "WireGuard",
            Self::OpenVpn => "OpenVPN",
            Self::OpenConnect => "OpenConnect",
            Self::StrongSwan => "IPsec",
            Self::Pptp => "PPTP",
            Self::L2tp => "L2TP",
            Self::Other(name) => name,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Vpn {
    pub uuid: String,
    pub name: String,
    pub kind: VpnKind,
    pub state: LinkState,
    pub interface: Option<String>,
    pub gateway: Option<String>,
    pub user: Option<String>,
    pub ip4: Option<String>,
    pub autoconnect: bool,
}

#[derive(Clone, Debug, Default)]
pub struct BluetoothDevice {
    pub address: String,
    pub name: String,
    pub nap: bool,
    pub dun: bool,
    pub state: LinkState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileKind {
    Wifi,
    Ethernet,
    Vpn,
    WireGuard,
    Bluetooth,
    Mobile,
    Bridge,
    Bond,
    Vlan,
    Loopback,
    Other(String),
}

impl ProfileKind {
    pub fn from_nm(kind: &str) -> Self {
        match kind {
            "802-11-wireless" => Self::Wifi,
            "802-3-ethernet" => Self::Ethernet,
            "vpn" => Self::Vpn,
            "wireguard" => Self::WireGuard,
            "bluetooth" => Self::Bluetooth,
            "gsm" | "cdma" => Self::Mobile,
            "bridge" => Self::Bridge,
            "bond" => Self::Bond,
            "vlan" => Self::Vlan,
            "loopback" => Self::Loopback,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Wifi => "Wi-Fi",
            Self::Ethernet => "Ethernet",
            Self::Vpn => "VPN",
            Self::WireGuard => "WireGuard",
            Self::Bluetooth => "Bluetooth",
            Self::Mobile => "Mobile",
            Self::Bridge => "Bridge",
            Self::Bond => "Bond",
            Self::Vlan => "VLAN",
            Self::Loopback => "Loopback",
            Self::Other(kind) => kind,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub uuid: String,
    pub name: String,
    pub kind: ProfileKind,
    pub interface: Option<String>,
    pub autoconnect: bool,
    pub priority: i32,
    pub last_used: u64,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IpMethod {
    #[default]
    Auto,
    Manual,
    LinkLocal,
    Shared,
    Disabled,
}

impl IpMethod {
    pub const ALL: [IpMethod; 5] = [
        Self::Auto,
        Self::Manual,
        Self::LinkLocal,
        Self::Shared,
        Self::Disabled,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Manual => "Manual",
            Self::LinkLocal => "Link-local",
            Self::Shared => "Shared",
            Self::Disabled => "Disabled",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IpConfig {
    pub method: IpMethod,
    pub addresses: Vec<String>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    pub search: Vec<String>,
    pub ignore_auto_dns: bool,
    pub never_default: bool,
}

#[derive(Clone, Debug)]
pub struct ProfileDetail {
    pub profile: Profile,
    pub ipv4: IpConfig,
    pub ipv6: IpConfig,
    pub mac_randomization: Option<String>,
    pub metered: Option<bool>,
    pub sections: Vec<(String, Vec<(String, String)>)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfileEdit {
    pub name: String,
    pub interface: Option<String>,
    pub autoconnect: bool,
    pub priority: i32,
    pub ipv4: IpConfig,
    pub ipv6: IpConfig,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlobalDns {
    pub searches: Vec<String>,
    pub servers: Vec<String>,
    pub domains: Vec<(String, Vec<String>)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EapMethod {
    #[default]
    Peap,
    Ttls,
    Tls,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InnerAuth {
    #[default]
    Mschapv2,
    Pap,
}

#[derive(Clone, Debug, Default)]
pub struct Enterprise {
    pub method: EapMethod,
    pub inner: InnerAuth,
    pub identity: String,
    pub password: String,
    pub anonymous_identity: Option<String>,
    pub domain: Option<String>,
    pub ca_cert: Option<String>,
    pub system_ca: bool,
    pub client_cert: Option<String>,
    pub private_key: Option<String>,
    pub private_key_password: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Credentials {
    None,
    Password(String),
    Enterprise(Box<Enterprise>),
}

#[derive(Clone, Debug, Default)]
pub struct WireGuardForm {
    pub name: String,
    pub private_key: String,
    pub address: String,
    pub dns: Vec<String>,
    pub mtu: Option<u32>,
    pub peer_public_key: String,
    pub endpoint: String,
    pub allowed_ips: Vec<String>,
    pub preshared_key: Option<String>,
    pub keepalive: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct HotspotForm {
    pub ssid: String,
    pub password: String,
    pub interface: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Action {
    Scan,
    SetWifi(bool),
    SetWwan(bool),
    SetBluetooth(bool),
    SetAirplane(bool),
    Connect {
        ssid: String,
        interface: Option<String>,
        bssid: Option<String>,
        security: Security,
        credentials: Credentials,
        hidden: bool,
    },
    Activate(String),
    Deactivate(String),
    DisconnectDevice(String),
    DeleteProfile(String),
    UpdateProfile(String, Box<ProfileEdit>),
    SetProfileAutoconnect(String, bool),
    ConnectBluetooth {
        address: String,
        name: String,
        dun: bool,
    },
    ImportOvpn {
        path: String,
        user: Option<String>,
        password: Option<String>,
    },
    AddWireGuard(Box<WireGuardForm>),
    StartHotspot(HotspotForm),
    SetManaged(String, bool),
    SetDeviceAutoconnect(String, bool),
    SetGlobalDns(GlobalDns),
    CheckConnectivity,
}

impl Action {
    pub fn describe(&self) -> String {
        match self {
            Self::Scan => "Scan".into(),
            Self::SetWifi(on) => format!("Wi-Fi {}", on_off(*on)),
            Self::SetWwan(on) => format!("Mobile broadband {}", on_off(*on)),
            Self::SetBluetooth(on) => format!("Bluetooth {}", on_off(*on)),
            Self::SetAirplane(on) => format!("Airplane mode {}", on_off(*on)),
            Self::Connect { ssid, .. } => format!("Connect to {ssid}"),
            Self::Activate(_) => "Activate".into(),
            Self::Deactivate(_) => "Deactivate".into(),
            Self::DisconnectDevice(iface) => format!("Disconnect {iface}"),
            Self::DeleteProfile(_) => "Forget".into(),
            Self::UpdateProfile(..) => "Save profile".into(),
            Self::SetProfileAutoconnect(_, on) => format!("Autoconnect {}", on_off(*on)),
            Self::ConnectBluetooth { name, .. } => format!("Connect to {name}"),
            Self::ImportOvpn { .. } => "Import OpenVPN".into(),
            Self::AddWireGuard(form) => format!("Add {}", form.name),
            Self::StartHotspot(form) => format!("Hotspot {}", form.ssid),
            Self::SetManaged(iface, on) => format!("{iface} managed {}", on_off(*on)),
            Self::SetDeviceAutoconnect(iface, on) => {
                format!("{iface} autoconnect {}", on_off(*on))
            }
            Self::SetGlobalDns(_) => "Global DNS".into(),
            Self::CheckConnectivity => "Connectivity check".into(),
        }
    }
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_frequencies_to_bands() {
        assert_eq!(band(2437), Some("2.4 GHz"));
        assert_eq!(band(5180), Some("5 GHz"));
        assert_eq!(band(6115), Some("6 GHz"));
        assert_eq!(band(60480), None);
    }

    #[test]
    fn maps_frequencies_to_channels() {
        assert_eq!(channel(2412), Some(1));
        assert_eq!(channel(2484), Some(14));
        assert_eq!(channel(5180), Some(36));
        assert_eq!(channel(5745), Some(149));
        assert_eq!(channel(5955), Some(1));
    }

    #[test]
    fn detects_virtual_wired_devices() {
        let mut dev = WiredDevice {
            interface: "veth3b063a8".into(),
            ..Default::default()
        };
        assert!(dev.is_virtual());
        dev.interface = "enp39s0".into();
        dev.driver = Some("r8169".into());
        assert!(!dev.is_virtual());
        dev.driver = Some("bridge".into());
        assert!(dev.is_virtual());
    }

    #[test]
    fn classifies_profile_kinds() {
        assert_eq!(ProfileKind::from_nm("802-11-wireless"), ProfileKind::Wifi);
        assert_eq!(ProfileKind::from_nm("wireguard"), ProfileKind::WireGuard);
        assert_eq!(
            ProfileKind::from_nm("tun"),
            ProfileKind::Other("tun".into())
        );
    }
}
