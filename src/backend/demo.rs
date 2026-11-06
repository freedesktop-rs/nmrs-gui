use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::channel::mpsc;
use futures::future::BoxFuture;

use super::Service;
use crate::model::*;

pub struct Demo {
    state: Arc<Mutex<State>>,
    notify: Arc<Mutex<Vec<mpsc::UnboundedSender<()>>>>,
}

impl Demo {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(seed())),
            notify: Arc::default(),
        }
    }
}

impl Service for Demo {
    fn state(&self) -> BoxFuture<'static, Result<State>> {
        let state = self.state.clone();
        Box::pin(async move { Ok(state.lock().map_err(|_| anyhow!("poisoned"))?.clone()) })
    }

    fn profile(&self, uuid: String) -> BoxFuture<'static, Result<ProfileDetail>> {
        let state = self.state.clone();
        Box::pin(async move {
            let state = state.lock().map_err(|_| anyhow!("poisoned"))?;
            let profile = state
                .profiles
                .iter()
                .find(|p| p.uuid == uuid)
                .cloned()
                .ok_or_else(|| anyhow!("profile not found"))?;
            Ok(detail(profile))
        })
    }

    fn execute(&self, action: Action) -> BoxFuture<'static, Result<()>> {
        let state = self.state.clone();
        let notify = self.notify.clone();
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(450)).await;
            {
                let mut state = state.lock().map_err(|_| anyhow!("poisoned"))?;
                apply(&mut state, action)?;
            }
            if let Ok(mut senders) = notify.lock() {
                senders.retain(|tx| tx.unbounded_send(()).is_ok());
            }
            Ok(())
        })
    }

    fn watch(&self, tx: mpsc::UnboundedSender<()>) -> BoxFuture<'static, Result<()>> {
        if let Ok(mut senders) = self.notify.lock() {
            senders.push(tx);
        }
        Box::pin(async { Ok(()) })
    }
}

fn apply(state: &mut State, action: Action) -> Result<()> {
    match action {
        Action::Scan | Action::CheckConnectivity => {}
        Action::Reconnect(uuid, _) => {
            for net in &mut state.networks {
                if net.profile.as_deref() == Some(uuid.as_str()) {
                    net.active = true;
                    net.ip4 = Some("192.168.1.42/24".into());
                }
            }
        }
        Action::SetWifi(on) => {
            state.radios.wifi.enabled = on;
            if !on {
                state.networks.iter_mut().for_each(|n| n.active = false);
            }
        }
        Action::SetWwan(on) => state.radios.wwan.enabled = on,
        Action::SetBluetooth(on) => state.radios.bluetooth.enabled = on,
        Action::SetAirplane(on) => {
            state.airplane = on;
            state.radios.wifi.enabled = !on;
            state.radios.bluetooth.enabled = !on;
            state.radios.wwan.enabled = !on;
        }
        Action::Connect {
            ssid, credentials, ..
        } => {
            if let Credentials::Password(pw) = &credentials
                && pw.len() < 8
            {
                return Err(anyhow!("Secrets were required, but not provided"));
            }
            for net in &mut state.networks {
                net.active = net.ssid == ssid;
                if net.active {
                    net.known = true;
                    net.ip4 = Some("192.168.1.42/24".into());
                    if net.profile.is_none() {
                        let uuid = format!("demo-{}", net.ssid.to_lowercase().replace(' ', "-"));
                        net.profile = Some(uuid.clone());
                        state.profiles.push(Profile {
                            uuid,
                            name: net.ssid.clone(),
                            kind: ProfileKind::Wifi,
                            interface: None,
                            autoconnect: true,
                            priority: 0,
                            last_used: 1_793_000_000,
                            active: true,
                        });
                    }
                } else {
                    net.ip4 = None;
                }
            }
        }
        Action::Activate(uuid) | Action::Deactivate(uuid) => {
            let on = state.profiles.iter().any(|p| p.uuid == uuid && !p.active);
            if let Some(vpn) = state.vpns.iter_mut().find(|v| v.uuid == uuid) {
                vpn.state = if on {
                    LinkState::Connected
                } else {
                    LinkState::Disconnected
                };
                vpn.ip4 = on.then(|| "10.66.0.2/32".into());
            }
            for net in &mut state.networks {
                if net.profile.as_deref() == Some(uuid.as_str()) {
                    net.active = on;
                } else if on
                    && state
                        .profiles
                        .iter()
                        .any(|p| p.uuid == uuid && p.kind == ProfileKind::Wifi)
                {
                    net.active = false;
                }
            }
            for profile in &mut state.profiles {
                if profile.uuid == uuid {
                    profile.active = on;
                }
            }
        }
        Action::DisconnectDevice(_) => state.networks.iter_mut().for_each(|n| n.active = false),
        Action::DeleteProfile(uuid) => {
            state.profiles.retain(|p| p.uuid != uuid);
            state.vpns.retain(|v| v.uuid != uuid);
            for net in &mut state.networks {
                if net.profile.as_deref() == Some(uuid.as_str()) {
                    net.profile = None;
                    net.known = false;
                    net.active = false;
                }
            }
        }
        Action::UpdateProfile(uuid, edit) => {
            if let Some(p) = state.profiles.iter_mut().find(|p| p.uuid == uuid) {
                p.name = edit.name.clone();
                p.autoconnect = edit.autoconnect;
                p.priority = edit.priority;
                p.interface = edit.interface.clone();
            }
        }
        Action::SetProfileAutoconnect(uuid, on) => {
            for p in state.profiles.iter_mut().filter(|p| p.uuid == uuid) {
                p.autoconnect = on;
            }
            for v in state.vpns.iter_mut().filter(|v| v.uuid == uuid) {
                v.autoconnect = on;
            }
        }
        Action::ConnectBluetooth { address, .. } => {
            for dev in &mut state.bluetooth {
                if dev.address == address {
                    dev.state = LinkState::Connected;
                }
            }
        }
        Action::ImportOvpn { path, .. } => {
            let name = std::path::Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "imported".into());
            add_vpn(
                state,
                name,
                VpnKind::OpenVpn,
                Some("vpn.example.net:1194".into()),
            );
        }
        Action::AddWireGuard(form) => {
            add_vpn(
                state,
                form.name.clone(),
                VpnKind::WireGuard,
                Some(form.endpoint.clone()),
            );
        }
        Action::StartHotspot(_) => {}
        Action::SetManaged(iface, on) => {
            for dev in &mut state.wired {
                if dev.interface == iface {
                    dev.managed = on;
                }
            }
        }
        Action::SetDeviceAutoconnect(iface, on) => {
            for dev in &mut state.wired {
                if dev.interface == iface {
                    dev.autoconnect = on;
                }
            }
        }
        Action::SetGlobalDns(dns) => state.dns = dns,
    }
    Ok(())
}

fn add_vpn(state: &mut State, name: String, kind: VpnKind, gateway: Option<String>) {
    let uuid = format!("demo-vpn-{}", state.vpns.len() + 1);
    state.profiles.push(Profile {
        uuid: uuid.clone(),
        name: name.clone(),
        kind: if kind == VpnKind::WireGuard {
            ProfileKind::WireGuard
        } else {
            ProfileKind::Vpn
        },
        interface: None,
        autoconnect: false,
        priority: 0,
        last_used: 0,
        active: false,
    });
    state.vpns.push(Vpn {
        uuid,
        name,
        kind,
        state: LinkState::Disconnected,
        interface: None,
        gateway,
        user: None,
        ip4: None,
        autoconnect: false,
    });
}

fn detail(profile: Profile) -> ProfileDetail {
    let manual = profile.name == "Studio";
    let ipv4 = if manual {
        IpConfig {
            method: IpMethod::Manual,
            addresses: vec!["10.0.40.12/24".into()],
            gateway: Some("10.0.40.1".into()),
            dns: vec!["10.0.40.1".into(), "9.9.9.9".into()],
            search: vec!["studio.lan".into()],
            ignore_auto_dns: true,
            never_default: false,
        }
    } else {
        IpConfig::default()
    };
    let kind = match profile.kind {
        ProfileKind::Wifi => "802-11-wireless",
        ProfileKind::Ethernet => "802-3-ethernet",
        ProfileKind::WireGuard => "wireguard",
        ProfileKind::Vpn => "vpn",
        _ => "generic",
    };
    let mut sections = vec![(
        "connection".to_string(),
        vec![
            ("autoconnect".into(), profile.autoconnect.to_string()),
            ("autoconnect-priority".into(), profile.priority.to_string()),
            ("id".into(), profile.name.clone()),
            ("type".into(), kind.into()),
            ("uuid".into(), profile.uuid.clone()),
        ],
    )];
    if profile.kind == ProfileKind::Wifi {
        sections.push((
            "802-11-wireless".into(),
            vec![
                ("mac-address-randomization".into(), "0".into()),
                ("mode".into(), "infrastructure".into()),
                ("ssid".into(), profile.name.clone()),
            ],
        ));
        sections.push((
            "802-11-wireless-security".into(),
            vec![
                ("key-mgmt".into(), "wpa-psk".into()),
                ("psk-flags".into(), "0".into()),
            ],
        ));
    }
    sections.push((
        "ipv4".into(),
        vec![(
            "method".into(),
            if manual { "manual" } else { "auto" }.into(),
        )],
    ));
    sections.push(("ipv6".into(), vec![("method".into(), "auto".into())]));
    ProfileDetail {
        mac_randomization: (profile.kind == ProfileKind::Wifi).then(|| "default".into()),
        metered: None,
        profile,
        ipv4,
        ipv6: IpConfig::default(),
        sections,
    }
}

fn ap(bssid: &str, strength: u8, frequency: u32, rate: u32, active: bool) -> AccessPoint {
    AccessPoint {
        bssid: bssid.into(),
        strength,
        frequency,
        rate_mbps: rate,
        active,
    }
}

fn net(
    ssid: &str,
    security: Security,
    aps: Vec<AccessPoint>,
    known: Option<&str>,
    active: bool,
) -> WifiNetwork {
    let best = aps
        .iter()
        .max_by_key(|a| a.strength)
        .cloned()
        .unwrap_or_default();
    WifiNetwork {
        ssid: ssid.into(),
        interface: "wlan0".into(),
        strength: best.strength,
        frequency: best.frequency,
        security,
        active,
        known: known.is_some(),
        profile: known.map(Into::into),
        access_points: aps,
        ip4: active.then(|| "192.168.1.42/24".into()),
        ip6: active.then(|| "fd00:1:2::42/64".into()),
    }
}

fn profile(uuid: &str, name: &str, kind: ProfileKind, last_used: u64, active: bool) -> Profile {
    Profile {
        uuid: uuid.into(),
        name: name.into(),
        kind,
        interface: None,
        autoconnect: true,
        priority: 0,
        last_used,
        active,
    }
}

fn seed() -> State {
    let on = Radio {
        enabled: true,
        hardware: true,
        present: true,
    };
    State {
        radios: Radios {
            wifi: on,
            wwan: Radio::default(),
            bluetooth: on,
        },
        airplane: false,
        connectivity: Connectivity {
            state: Reachability::Full,
            checking: true,
            uri: Some("http://ping.archlinux.org/nm-check.txt".into()),
            portal: None,
        },
        wifi_devices: vec![WifiDevice {
            interface: "wlan0".into(),
            mac: "3C:A9:F4:1E:22:90".into(),
            driver: Some("iwlwifi".into()),
            state: LinkState::Connected,
            active_ssid: Some("Halcyon".into()),
            managed: true,
        }],
        networks: vec![
            net(
                "Halcyon",
                Security::Sae,
                vec![
                    ap("A4:2B:B0:11:3C:01", 86, 5180, 1201, true),
                    ap("A4:2B:B0:11:3C:00", 74, 2437, 286, false),
                    ap("A4:2B:B0:11:3C:05", 41, 5745, 866, false),
                ],
                Some("a1"),
                true,
            ),
            net(
                "Halcyon Guest",
                Security::Psk,
                vec![ap("A4:2B:B0:11:3C:02", 79, 2437, 286, false)],
                Some("a2"),
                false,
            ),
            net(
                "eduroam",
                Security::Eap,
                vec![
                    ap("00:3A:7D:90:10:A1", 58, 5500, 866, false),
                    ap("00:3A:7D:90:10:A0", 47, 2462, 144, false),
                ],
                None,
                false,
            ),
            net(
                "Studio",
                Security::Psk,
                vec![ap("F0:9F:C2:7A:00:12", 52, 5240, 866, false)],
                Some("a3"),
                false,
            ),
            net(
                "NETGEAR-5G-8812",
                Security::Psk,
                vec![ap("9C:3D:CF:88:12:5B", 44, 5805, 1300, false)],
                None,
                false,
            ),
            net(
                "Lighthouse Café",
                Security::Open,
                vec![ap("BE:EF:00:12:34:56", 37, 2412, 144, false)],
                None,
                false,
            ),
            net(
                "DIRECT-roku-417",
                Security::Psk,
                vec![ap("D2:0A:11:7F:42:17", 29, 2462, 72, false)],
                None,
                false,
            ),
            net(
                "Pixel_7310",
                Security::Sae,
                vec![ap("2E:91:4B:00:73:10", 18, 6115, 2402, false)],
                None,
                false,
            ),
        ],
        wired: vec![
            WiredDevice {
                interface: "enp5s0".into(),
                mac: "D8:BB:C1:0A:5E:7C".into(),
                driver: Some("r8169".into()),
                speed_mbps: Some(2500),
                state: LinkState::Connected,
                profile: Some("Wired connection 1".into()),
                profile_uuid: Some("e1".into()),
                ip4: Some("192.168.1.20/24".into()),
                ip6: Some("fd00:1:2::20/64".into()),
                managed: true,
                autoconnect: true,
            },
            WiredDevice {
                interface: "enx00e04c680d1f".into(),
                mac: "00:E0:4C:68:0D:1F".into(),
                driver: Some("r8152".into()),
                speed_mbps: None,
                state: LinkState::Unavailable,
                profile: None,
                profile_uuid: None,
                ip4: None,
                ip6: None,
                managed: true,
                autoconnect: true,
            },
        ],
        vpns: vec![
            Vpn {
                uuid: "v1".into(),
                name: "mullvad-se-got".into(),
                kind: VpnKind::WireGuard,
                state: LinkState::Connected,
                interface: Some("mullvad-se-got".into()),
                gateway: Some("185.213.154.68:51820".into()),
                user: None,
                ip4: Some("10.66.12.181/32".into()),
                autoconnect: false,
            },
            Vpn {
                uuid: "v2".into(),
                name: "Work".into(),
                kind: VpnKind::OpenVpn,
                state: LinkState::Disconnected,
                interface: None,
                gateway: Some("vpn.corp.example:1194".into()),
                user: Some("akrm".into()),
                ip4: None,
                autoconnect: false,
            },
            Vpn {
                uuid: "v3".into(),
                name: "homelab".into(),
                kind: VpnKind::WireGuard,
                state: LinkState::Disconnected,
                interface: None,
                gateway: Some("home.example.net:51820".into()),
                user: None,
                ip4: None,
                autoconnect: false,
            },
        ],
        bluetooth: vec![
            BluetoothDevice {
                address: "64:A2:F9:3B:10:CE".into(),
                name: "Pixel 9".into(),
                nap: true,
                dun: true,
                state: LinkState::Disconnected,
            },
            BluetoothDevice {
                address: "F4:7B:09:C1:22:A0".into(),
                name: "ThinkPad Dock".into(),
                nap: true,
                dun: false,
                state: LinkState::Disconnected,
            },
        ],
        profiles: vec![
            profile("a1", "Halcyon", ProfileKind::Wifi, 1_793_570_000, true),
            profile(
                "a2",
                "Halcyon Guest",
                ProfileKind::Wifi,
                1_791_100_000,
                false,
            ),
            profile("a3", "Studio", ProfileKind::Wifi, 1_788_000_000, false),
            profile(
                "a4",
                "Airport_Free_WiFi",
                ProfileKind::Wifi,
                1_760_000_000,
                false,
            ),
            profile(
                "e1",
                "Wired connection 1",
                ProfileKind::Ethernet,
                1_793_570_000,
                true,
            ),
            profile(
                "v1",
                "mullvad-se-got",
                ProfileKind::WireGuard,
                1_793_560_000,
                true,
            ),
            profile("v2", "Work", ProfileKind::Vpn, 1_792_000_000, false),
            profile(
                "v3",
                "homelab",
                ProfileKind::WireGuard,
                1_790_000_000,
                false,
            ),
            profile(
                "b1",
                "Pixel 9 Network",
                ProfileKind::Bluetooth,
                1_770_000_000,
                false,
            ),
            profile("l1", "lo", ProfileKind::Loopback, 1_793_570_000, true),
        ],
        dns: GlobalDns {
            searches: vec![],
            servers: vec![],
            domains: vec![],
        },
    }
}
