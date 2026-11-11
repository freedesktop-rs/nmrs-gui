use std::time::Duration;

use futures::StreamExt;
use futures::channel::{mpsc, oneshot};
use ksni::blocking::TrayMethods;
use ksni::menu::{CheckmarkItem, MenuItem, StandardItem};

use crate::backend::Backend;
use crate::config::{Page, TrayClick};
use crate::ipc::{Command, Request};
use crate::model::{Action, State, WifiNetwork};
use crate::status::Status;

const NETWORKS: usize = 8;

struct NmTray<const MENU: bool> {
    state: State,
    backend: Backend,
    commands: mpsc::UnboundedSender<Request>,
}

impl<const MENU: bool> NmTray<MENU> {
    fn send(&self, command: Command) {
        let (reply, _) = oneshot::channel();
        let _ = self.commands.unbounded_send((command, reply));
    }

    fn status(&self) -> Status {
        Status::from_state(&self.state, None)
    }
}

fn icon(status: &Status) -> &'static str {
    if status.airplane {
        return "airplane-mode-symbolic";
    }
    if status.vpn.iter().any(|v| v.state == "connected") {
        return "network-vpn-symbolic";
    }
    match (&status.primary, &status.wifi) {
        (Some(primary), _) if primary.kind == "ethernet" => "network-wired-symbolic",
        (Some(_), Some(wifi)) => match wifi.strength {
            80.. => "network-wireless-signal-excellent-symbolic",
            55.. => "network-wireless-signal-good-symbolic",
            30.. => "network-wireless-signal-ok-symbolic",
            5.. => "network-wireless-signal-weak-symbolic",
            _ => "network-wireless-signal-none-symbolic",
        },
        _ if status.radios.wifi.present && !status.radios.wifi.enabled => {
            "network-wireless-disabled-symbolic"
        }
        _ => "network-offline-symbolic",
    }
}

fn summary(status: &Status) -> String {
    if status.airplane {
        return "Airplane mode".into();
    }
    let mut text = match (&status.primary, &status.wifi) {
        (Some(primary), Some(wifi)) if primary.kind == "wifi" => {
            format!("{} · {}%", wifi.ssid, wifi.strength)
        }
        (Some(primary), _) => primary.name.clone(),
        _ if status.radios.wifi.present && !status.radios.wifi.enabled => "Wi-Fi off".into(),
        _ => "Not connected".into(),
    };
    if status.connectivity == "portal" {
        text.push_str(" · sign-in required");
    }
    text
}

fn details(status: &Status) -> String {
    let mut lines = Vec::new();
    if let Some(wifi) = &status.wifi {
        let mut parts = vec![wifi.security.to_string()];
        parts.extend(wifi.band.map(str::to_string));
        parts.extend(wifi.ip4.clone());
        lines.push(parts.join(" · "));
    }
    for eth in &status.ethernet {
        let mut parts = vec![eth.interface.clone()];
        parts.extend(eth.speed_mbps.map(|s| format!("{s} Mb/s")));
        parts.extend(eth.ip4.clone());
        lines.push(parts.join(" · "));
    }
    for vpn in &status.vpn {
        lines.push(format!("{} · {} · {}", vpn.name, vpn.kind, vpn.state));
    }
    lines.join("\n")
}

fn networks(state: &State) -> Vec<&WifiNetwork> {
    let mut nets: Vec<&WifiNetwork> = state
        .networks
        .iter()
        .filter(|n| n.active || (n.known && n.profile.is_some()))
        .collect();
    nets.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then(b.strength.cmp(&a.strength))
            .then(a.ssid.cmp(&b.ssid))
    });
    nets.truncate(NETWORKS);
    nets
}

fn label(text: &str) -> String {
    text.replace('_', "__")
}

fn run<const MENU: bool>(action: Action) -> Box<dyn Fn(&mut NmTray<MENU>) + Send> {
    Box::new(move |tray| tray.backend.dispatch(action.clone()))
}

fn open<const MENU: bool>(command: Command) -> Box<dyn Fn(&mut NmTray<MENU>) + Send> {
    Box::new(move |tray| tray.send(command.clone()))
}

impl<const MENU: bool> ksni::Tray for NmTray<MENU> {
    const MENU_ON_ACTIVATE: bool = MENU;

    fn id(&self) -> String {
        "nmrs".into()
    }

    fn title(&self) -> String {
        "nmrs".into()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::Hardware
    }

    fn icon_name(&self) -> String {
        icon(&self.status()).into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let status = self.status();
        ksni::ToolTip {
            title: summary(&status),
            description: details(&status),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(Command::Toggle(None));
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        self.send(Command::Toggle(None));
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let state = &self.state;
        let mut items: Vec<MenuItem<Self>> = vec![
            StandardItem {
                label: label(&summary(&self.status())),
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
        ];

        if state.radios.wifi.present {
            for net in networks(state) {
                let action = match (net.active, net.profile.clone()) {
                    (true, Some(uuid)) => Action::Deactivate(uuid),
                    (true, None) => Action::DisconnectDevice(net.interface.clone()),
                    (false, Some(uuid)) => Action::Activate(uuid),
                    (false, None) => continue,
                };
                items.push(
                    CheckmarkItem {
                        label: label(&format!("{}  {}%", net.ssid, net.strength)),
                        checked: net.active,
                        activate: run(action),
                        ..Default::default()
                    }
                    .into(),
                );
            }
            items.push(
                StandardItem {
                    label: "More networks…".into(),
                    activate: open(Command::Show(Some(Page::Wifi))),
                    ..Default::default()
                }
                .into(),
            );
            items.push(MenuItem::Separator);
        }

        if !state.vpns.is_empty() {
            for vpn in &state.vpns {
                let up = vpn.state.is_up();
                let action = if up {
                    Action::Deactivate(vpn.uuid.clone())
                } else {
                    Action::Activate(vpn.uuid.clone())
                };
                items.push(
                    CheckmarkItem {
                        label: label(&vpn.name),
                        checked: up,
                        enabled: !vpn.state.is_busy(),
                        activate: run(action),
                        ..Default::default()
                    }
                    .into(),
                );
            }
            items.push(MenuItem::Separator);
        }

        if state.radios.wifi.present {
            items.push(
                CheckmarkItem {
                    label: "Wi-Fi".into(),
                    checked: state.radios.wifi.enabled,
                    enabled: state.radios.wifi.hardware,
                    activate: run(Action::SetWifi(!state.radios.wifi.enabled)),
                    ..Default::default()
                }
                .into(),
            );
        }
        items.push(
            CheckmarkItem {
                label: "Airplane mode".into(),
                checked: state.airplane,
                activate: run(Action::SetAirplane(!state.airplane)),
                ..Default::default()
            }
            .into(),
        );
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Open nmrs".into(),
                activate: open(Command::Show(None)),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Quit".into(),
                activate: open(Command::Quit),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

pub fn spawn(
    backend: Backend,
    commands: mpsc::UnboundedSender<Request>,
    click: TrayClick,
) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("nmrs-tray".into())
        .spawn(move || {
            let result = match click {
                TrayClick::Menu => serve::<true>(backend, commands),
                TrayClick::Window => serve::<false>(backend, commands),
            };
            if let Err(err) = result {
                log::warn!("tray stopped: {err}");
            }
        })?;
    Ok(())
}

fn serve<const MENU: bool>(
    backend: Backend,
    commands: mpsc::UnboundedSender<Request>,
) -> anyhow::Result<()> {
    let mut events = backend.watch();
    let state = futures::executor::block_on(backend.state()).unwrap_or_default();
    let handle = NmTray::<MENU> {
        state,
        backend: backend.clone(),
        commands,
    }
    .assume_sni_available(true)
    .spawn()?;
    while futures::executor::block_on(events.next()).is_some() {
        std::thread::sleep(Duration::from_millis(150));
        while let Ok(()) = events.try_recv() {}
        match futures::executor::block_on(backend.state()) {
            Ok(state) => {
                if handle.update(|tray| tray.state = state).is_none() {
                    break;
                }
            }
            Err(err) => log::warn!("tray refresh failed: {err}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LinkState, Reachability, Security, Vpn, VpnKind, WiredDevice};

    fn net(ssid: &str, strength: u8, active: bool, known: bool) -> WifiNetwork {
        WifiNetwork {
            ssid: ssid.into(),
            interface: "wlan0".into(),
            strength,
            frequency: 5180,
            security: Security::Sae,
            active,
            known,
            profile: known.then(|| format!("uuid-{ssid}")),
            ..Default::default()
        }
    }

    fn state() -> State {
        let mut state = State::default();
        state.connectivity.state = Reachability::Full;
        state.radios.wifi.present = true;
        state.radios.wifi.enabled = true;
        state.networks = vec![
            net("Cafe", 90, false, false),
            net("Office", 40, false, true),
            net("Halcyon", 62, true, true),
            net("Home_5G", 75, false, true),
        ];
        state
    }

    #[::core::prelude::v1::test]
    fn icon_follows_the_primary_connection() {
        let mut state = state();
        let status = Status::from_state(&state, None);
        assert_eq!(icon(&status), "network-wireless-signal-good-symbolic");
        assert_eq!(summary(&status), "Halcyon · 62%");

        state.wired.push(WiredDevice {
            interface: "enp5s0".into(),
            state: LinkState::Connected,
            ..Default::default()
        });
        let status = Status::from_state(&state, None);
        assert_eq!(icon(&status), "network-wired-symbolic");
        assert_eq!(summary(&status), "enp5s0");

        state.vpns.push(Vpn {
            uuid: "v1".into(),
            name: "mullvad".into(),
            kind: VpnKind::WireGuard,
            state: LinkState::Connected,
            interface: None,
            gateway: None,
            user: None,
            ip4: None,
            autoconnect: false,
        });
        assert_eq!(
            icon(&Status::from_state(&state, None)),
            "network-vpn-symbolic"
        );

        state.airplane = true;
        let status = Status::from_state(&state, None);
        assert_eq!(icon(&status), "airplane-mode-symbolic");
        assert_eq!(summary(&status), "Airplane mode");
    }

    #[::core::prelude::v1::test]
    fn offline_states_are_told_apart() {
        let mut state = State::default();
        state.radios.wifi.present = true;
        state.radios.wifi.enabled = true;
        let status = Status::from_state(&state, None);
        assert_eq!(icon(&status), "network-offline-symbolic");
        assert_eq!(summary(&status), "Not connected");

        state.radios.wifi.enabled = false;
        let status = Status::from_state(&state, None);
        assert_eq!(icon(&status), "network-wireless-disabled-symbolic");
        assert_eq!(summary(&status), "Wi-Fi off");
    }

    #[::core::prelude::v1::test]
    fn menu_lists_the_active_network_then_saved_ones_by_strength() {
        let state = state();
        let names: Vec<&str> = networks(&state).iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["Halcyon", "Home_5G", "Office"]);
        assert_eq!(label("Home_5G"), "Home__5G");
    }
}
