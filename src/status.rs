use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use futures::StreamExt;
use serde::Serialize;

use crate::backend::Backend;
use crate::config::Store;
use crate::model::{Reachability, State};
use crate::theme::Colors;

#[derive(Debug, PartialEq, Serialize)]
pub struct Status {
    pub connectivity: &'static str,
    pub portal: Option<String>,
    pub primary: Option<Primary>,
    pub airplane: bool,
    pub radios: Radios,
    pub wifi: Option<Wifi>,
    pub ethernet: Vec<Ethernet>,
    pub vpn: Vec<Vpn>,
    pub theme: Option<Colors>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Primary {
    pub kind: &'static str,
    pub name: String,
    pub interface: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Radio {
    pub enabled: bool,
    pub hardware: bool,
    pub present: bool,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Radios {
    pub wifi: Radio,
    pub wwan: Radio,
    pub bluetooth: Radio,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Wifi {
    pub ssid: String,
    pub interface: String,
    pub strength: u8,
    pub band: Option<&'static str>,
    pub security: &'static str,
    pub ip4: Option<String>,
    pub ip6: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Ethernet {
    pub interface: String,
    pub profile: Option<String>,
    pub speed_mbps: Option<u32>,
    pub ip4: Option<String>,
    pub ip6: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Vpn {
    pub name: String,
    pub kind: String,
    pub state: String,
    pub managed: bool,
    pub interface: Option<String>,
    pub ip4: Option<String>,
}

fn radio(r: crate::model::Radio) -> Radio {
    Radio {
        enabled: r.enabled,
        hardware: r.hardware,
        present: r.present,
    }
}

impl Status {
    pub fn from_state(state: &State, theme: Option<Colors>) -> Self {
        let active = state.networks.iter().find(|n| n.active);
        let wired_up = state
            .wired
            .iter()
            .find(|d| d.state.is_up() && !d.is_virtual());
        let primary = wired_up
            .map(|d| Primary {
                kind: "ethernet",
                name: d.profile.clone().unwrap_or_else(|| d.interface.clone()),
                interface: d.interface.clone(),
            })
            .or_else(|| {
                active.map(|n| Primary {
                    kind: "wifi",
                    name: n.ssid.clone(),
                    interface: n.interface.clone(),
                })
            });
        Self {
            connectivity: match state.connectivity.state {
                Reachability::Unknown => "unknown",
                Reachability::None => "offline",
                Reachability::Portal => "portal",
                Reachability::Limited => "limited",
                Reachability::Full => "online",
            },
            portal: state.connectivity.portal.clone(),
            primary,
            airplane: state.airplane,
            radios: Radios {
                wifi: radio(state.radios.wifi),
                wwan: radio(state.radios.wwan),
                bluetooth: radio(state.radios.bluetooth),
            },
            wifi: active.map(|n| Wifi {
                ssid: n.ssid.clone(),
                interface: n.interface.clone(),
                strength: n.strength,
                band: n.band(),
                security: n.security.label(),
                ip4: n.ip4.clone(),
                ip6: n.ip6.clone(),
            }),
            ethernet: state
                .wired
                .iter()
                .filter(|d| d.state.is_up() && !d.is_virtual())
                .map(|d| Ethernet {
                    interface: d.interface.clone(),
                    profile: d.profile.clone(),
                    speed_mbps: d.speed_mbps,
                    ip4: d.ip4.clone(),
                    ip6: d.ip6.clone(),
                })
                .collect(),
            vpn: state
                .vpns
                .iter()
                .filter(|v| v.state.is_up() || v.state.is_busy())
                .map(|v| Vpn {
                    name: v.name.clone(),
                    kind: v.kind.label().to_string(),
                    state: v.state.label().to_lowercase(),
                    managed: v.managed,
                    interface: v.interface.clone(),
                    ip4: v.ip4.clone(),
                })
                .collect(),
            theme,
        }
    }
}

pub fn run(demo: bool, config: Option<PathBuf>, watch: bool) -> anyhow::Result<()> {
    let backend = Backend::start(demo)?;
    let appearance = Store::new(config)
        .load()
        .map(|c| c.appearance)
        .unwrap_or_default();
    let theme = crate::theme::colors_for(&appearance);
    let mut events = backend.watch();
    let mut out = std::io::stdout().lock();
    let mut last = None;
    loop {
        let state = futures::executor::block_on(backend.state())?;
        let status = Status::from_state(&state, theme.clone());
        if last.as_ref() != Some(&status) {
            let line = serde_json::to_string(&status)?;
            if writeln!(out, "{line}").and_then(|_| out.flush()).is_err() {
                return Ok(());
            }
            last = Some(status);
        }
        if !watch || futures::executor::block_on(events.next()).is_none() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(150));
        while let Ok(()) = events.try_recv() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LinkState, Security, Vpn as VpnModel, VpnKind, WifiNetwork, WiredDevice};

    fn state() -> State {
        let mut state = State::default();
        state.connectivity.state = Reachability::Full;
        state.networks.push(WifiNetwork {
            ssid: "Halcyon".into(),
            interface: "wlan0".into(),
            strength: 82,
            frequency: 5180,
            security: Security::Sae,
            active: true,
            known: true,
            ip4: Some("10.0.0.5/24".into()),
            ..Default::default()
        });
        state.vpns.push(VpnModel {
            uuid: "v1".into(),
            name: "mullvad".into(),
            kind: VpnKind::WireGuard,
            state: LinkState::Connected,
            interface: Some("mullvad".into()),
            gateway: None,
            user: None,
            ip4: None,
            autoconnect: false,
            managed: true,
        });
        state
    }

    #[::core::prelude::v1::test]
    fn wifi_is_primary_without_wired() {
        let status = Status::from_state(&state(), None);
        assert_eq!(status.connectivity, "online");
        assert_eq!(status.primary.as_ref().map(|p| p.kind), Some("wifi"));
        let wifi = status.wifi.unwrap();
        assert_eq!(wifi.band, Some("5 GHz"));
        assert_eq!(wifi.strength, 82);
        assert_eq!(status.vpn.len(), 1);
        assert_eq!(status.vpn[0].state, "connected");
    }

    #[::core::prelude::v1::test]
    fn unmanaged_vpn_is_reported() {
        let mut state = state();
        state.vpns.push(VpnModel {
            uuid: "unmanaged:tun0".into(),
            name: "tun0".into(),
            kind: VpnKind::OpenVpn,
            state: LinkState::Connected,
            interface: Some("tun0".into()),
            gateway: None,
            user: None,
            ip4: None,
            autoconnect: false,
            managed: false,
        });
        let status = Status::from_state(&state, None);
        assert_eq!(status.vpn.len(), 2);
        assert!(status.vpn[0].managed);
        assert!(!status.vpn[1].managed);
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"managed\":false"));
    }

    #[::core::prelude::v1::test]
    fn wired_wins_over_wifi() {
        let mut state = state();
        state.wired.push(WiredDevice {
            interface: "enp5s0".into(),
            state: LinkState::Connected,
            profile: Some("Wired connection 1".into()),
            ..Default::default()
        });
        let status = Status::from_state(&state, None);
        let primary = status.primary.unwrap();
        assert_eq!(primary.kind, "ethernet");
        assert_eq!(primary.name, "Wired connection 1");
        assert_eq!(status.ethernet.len(), 1);
    }

    #[::core::prelude::v1::test]
    fn serializes_to_flat_json() {
        let json = serde_json::to_value(Status::from_state(&state(), None)).unwrap();
        assert_eq!(json["connectivity"], "online");
        assert_eq!(json["wifi"]["ssid"], "Halcyon");
        assert_eq!(json["radios"]["wifi"]["enabled"], false);
        assert!(json["theme"].is_null());
    }
}
