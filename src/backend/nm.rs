use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use futures::StreamExt;
use futures::channel::mpsc;
use futures::future::BoxFuture;
use nmrs::builders::{WifiConnectionBuilder, WifiMode};
use nmrs::models::{IpMethod as NmIpMethod, IpSettings, SettingsSummary};
use nmrs::raw::zvariant::{OwnedValue, Str, Value};
use nmrs::{
    AccessPoint as NmAccessPoint, ActiveConnection, BluetoothIdentity, BluetoothNetworkRole,
    ConnectByUuidConfig, ConnectivityState, DeviceState, EapOptions, GlobalDnsConfiguration,
    GlobalDnsDomain, NetworkManager, SecurityFeatures, SettingsPatch, VpnType, WifiSecurity,
    WireGuardConfig, WireGuardPeer,
};

use super::Service;
use crate::model::*;

pub struct Nm {
    nm: Arc<NetworkManager>,
}

impl Nm {
    pub async fn connect() -> Result<Self> {
        let nm = NetworkManager::new()
            .await
            .context("NetworkManager is not reachable over D-Bus")?;
        Ok(Self { nm: Arc::new(nm) })
    }
}

impl Service for Nm {
    fn state(&self) -> BoxFuture<'static, Result<State>> {
        let nm = self.nm.clone();
        Box::pin(async move { load(&nm).await })
    }

    fn profile(&self, uuid: String) -> BoxFuture<'static, Result<ProfileDetail>> {
        let nm = self.nm.clone();
        Box::pin(async move { load_profile(&nm, &uuid).await })
    }

    fn execute(&self, action: Action) -> BoxFuture<'static, Result<()>> {
        let nm = self.nm.clone();
        Box::pin(async move { execute(&nm, action).await })
    }

    fn watch(&self, tx: mpsc::UnboundedSender<()>) -> BoxFuture<'static, Result<()>> {
        let nm = self.nm.clone();
        Box::pin(async move {
            let mut events = nm.network_events().await?;
            while events.next().await.is_some() {
                if tx.unbounded_send(()).is_err() {
                    break;
                }
            }
            Ok(())
        })
    }
}

async fn load(nm: &NetworkManager) -> Result<State> {
    let snapshot = nm.snapshot().await?;
    let active = &snapshot.active_connections;
    let active_uuids: Vec<&str> = active.iter().map(active_uuid).collect();

    let networks = snapshot
        .wifi_groups()
        .into_iter()
        .filter(|group| !group.ssid.is_empty())
        .map(|group| {
            let wifi_active = active.iter().find_map(|conn| match conn {
                ActiveConnection::Wifi(wifi) if wifi.ssid == group.ssid => Some(wifi),
                _ => None,
            });
            WifiNetwork {
                security: security(&group.strongest.security),
                strength: group.strongest.strength,
                frequency: group.strongest.frequency_mhz,
                interface: group.interface.clone(),
                active: group.active,
                known: group.known,
                profile: group.saved_profiles.first().map(|p| p.uuid.clone()),
                access_points: group.access_points.iter().map(access_point).collect(),
                ip4: wifi_active.and_then(|w| w.ip4_address.clone()),
                ip6: wifi_active.and_then(|w| w.ip6_address.clone()),
                ssid: group.ssid,
            }
        })
        .collect();

    let wifi_devices = snapshot
        .wifi_devices
        .iter()
        .map(|dev| WifiDevice {
            interface: dev.interface.clone(),
            mac: dev.hw_address.clone(),
            driver: dev.driver.clone(),
            state: link(&dev.state),
            active_ssid: dev.active_ssid.clone(),
            managed: dev.managed,
            autoconnect: dev.autoconnect,
        })
        .collect();

    let details = nm.list_wired_device_details().await.unwrap_or_default();
    let wired = snapshot
        .wired_devices
        .iter()
        .map(|dev| {
            let detail = details.iter().find(|d| d.interface == dev.interface);
            let conn = active.iter().find_map(|conn| match conn {
                ActiveConnection::Wired(w) if w.interface.as_deref() == Some(&dev.interface) => {
                    Some(w)
                }
                _ => None,
            });
            WiredDevice {
                interface: dev.interface.clone(),
                mac: dev.identity.current_mac.clone(),
                driver: dev.driver.clone(),
                speed_mbps: dev.speed_mbps.or(detail.and_then(|d| d.speed_mbps)),
                state: link(&dev.state),
                profile: conn
                    .map(|c| c.id.clone())
                    .or(detail.and_then(|d| d.active_connection_id.clone())),
                profile_uuid: conn.map(|c| c.uuid.clone()),
                ip4: dev.ip4_address.clone(),
                ip6: dev.ip6_address.clone(),
                managed: dev.managed.unwrap_or(true),
                autoconnect: dev.autoconnect.unwrap_or(true),
            }
        })
        .collect();

    let vpns = nm
        .list_vpn_connections()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|vpn| {
            let (kind, gateway) = vpn_kind(&vpn.vpn_type);
            let ip4 = active
                .iter()
                .find(|conn| active_uuid(conn) == vpn.uuid)
                .and_then(active_ip4);
            let autoconnect = snapshot
                .saved_connections
                .iter()
                .find(|p| p.uuid == vpn.uuid)
                .is_some_and(|p| p.autoconnect);
            let state = if active_uuids.contains(&vpn.uuid.as_str()) && !vpn.active {
                LinkState::Connecting
            } else if vpn.active {
                LinkState::Connected
            } else {
                link(&vpn.state)
            };
            Vpn {
                uuid: vpn.uuid,
                name: vpn.name,
                kind,
                state,
                interface: vpn.interface,
                gateway,
                user: vpn.user_name,
                ip4,
                autoconnect,
            }
        })
        .collect();

    let bluetooth = nm
        .list_bluetooth_devices()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|dev| BluetoothDevice {
            name: dev
                .alias
                .clone()
                .or(dev.name.clone())
                .unwrap_or_else(|| dev.bdaddr.clone()),
            nap: dev.bt_caps & 0x2 != 0,
            dun: dev.bt_caps & 0x1 != 0,
            state: link(&dev.state),
            address: dev.bdaddr,
        })
        .collect();

    let profiles = snapshot
        .saved_connections
        .iter()
        .map(|p| Profile {
            uuid: p.uuid.clone(),
            name: p.id.clone(),
            kind: ProfileKind::from_nm(&p.connection_type),
            interface: p.interface_name.clone(),
            autoconnect: p.autoconnect,
            priority: p.autoconnect_priority,
            last_used: p.timestamp_unix,
            active: active_uuids.contains(&p.uuid.as_str()),
        })
        .collect();

    let dns = nm
        .global_dns_configuration()
        .await
        .map(|cfg| global_dns(&cfg))
        .unwrap_or_default();

    let radio = |r: &nmrs::RadioState| Radio {
        enabled: r.enabled,
        hardware: r.hardware_enabled,
        present: r.present,
    };
    let airplane = &snapshot.airplane_mode;

    Ok(State {
        radios: Radios {
            wifi: radio(&snapshot.wifi),
            wwan: radio(&snapshot.wwan),
            bluetooth: radio(&snapshot.bluetooth),
        },
        airplane: [&airplane.wifi, &airplane.wwan, &airplane.bluetooth]
            .iter()
            .filter(|r| r.present)
            .all(|r| !r.enabled)
            && airplane.wifi.present,
        connectivity: Connectivity {
            state: match snapshot.connectivity.state {
                ConnectivityState::Full => Reachability::Full,
                ConnectivityState::Limited => Reachability::Limited,
                ConnectivityState::Portal => Reachability::Portal,
                ConnectivityState::None => Reachability::None,
                _ => Reachability::Unknown,
            },
            checking: snapshot.connectivity.check_enabled,
            uri: snapshot.connectivity.check_uri.clone(),
            portal: snapshot.connectivity.captive_portal_url.clone(),
        },
        wifi_devices,
        networks,
        wired,
        vpns,
        bluetooth,
        profiles,
        dns,
    })
}

fn active_uuid(conn: &ActiveConnection) -> &str {
    match conn {
        ActiveConnection::Wired(c) => &c.uuid,
        ActiveConnection::Wifi(c) => &c.uuid,
        ActiveConnection::Vpn(c) => &c.uuid,
        ActiveConnection::Other(c) => &c.uuid,
        _ => "",
    }
}

fn active_ip4(conn: &ActiveConnection) -> Option<String> {
    match conn {
        ActiveConnection::Wired(c) => c.ip4_address.clone(),
        ActiveConnection::Wifi(c) => c.ip4_address.clone(),
        ActiveConnection::Vpn(c) => c.ip4_address.clone(),
        ActiveConnection::Other(c) => c.ip4_address.clone(),
        _ => None,
    }
}

fn link(state: &DeviceState) -> LinkState {
    match state {
        DeviceState::Unmanaged => LinkState::Unmanaged,
        DeviceState::Unavailable => LinkState::Unavailable,
        DeviceState::Disconnected => LinkState::Disconnected,
        DeviceState::Prepare
        | DeviceState::Config
        | DeviceState::NeedAuth
        | DeviceState::IpConfig
        | DeviceState::IpCheck
        | DeviceState::Secondaries => LinkState::Connecting,
        DeviceState::Activated => LinkState::Connected,
        DeviceState::Deactivating => LinkState::Disconnecting,
        DeviceState::Failed => LinkState::Failed,
        _ => LinkState::Disconnected,
    }
}

fn security(features: &SecurityFeatures) -> Security {
    if features.eap || features.eap_suite_b_192 {
        Security::Eap
    } else if features.psk {
        Security::Psk
    } else if features.sae {
        Security::Sae
    } else if features.wep40 || features.wep104 {
        Security::Wep
    } else if features.owe || features.owe_transition_mode {
        Security::Owe
    } else if features.privacy {
        Security::Wep
    } else {
        Security::Open
    }
}

fn access_point(ap: &NmAccessPoint) -> AccessPoint {
    AccessPoint {
        bssid: ap.bssid.clone(),
        strength: ap.strength,
        frequency: ap.frequency_mhz,
        rate_mbps: ap.max_bitrate_kbps / 1000,
        active: ap.is_active,
    }
}

fn vpn_kind(kind: &VpnType) -> (VpnKind, Option<String>) {
    match kind {
        VpnType::WireGuard { endpoint, .. } => (VpnKind::WireGuard, endpoint.clone()),
        VpnType::OpenVpn { remote, .. } => (VpnKind::OpenVpn, remote.clone()),
        VpnType::OpenConnect { gateway, .. } => (VpnKind::OpenConnect, gateway.clone()),
        VpnType::StrongSwan { address, .. } => (VpnKind::StrongSwan, address.clone()),
        VpnType::Pptp { gateway, .. } => (VpnKind::Pptp, gateway.clone()),
        VpnType::L2tp { gateway, .. } => (VpnKind::L2tp, gateway.clone()),
        VpnType::Generic { service_type, .. } => (
            VpnKind::Other(
                service_type
                    .rsplit('.')
                    .next()
                    .unwrap_or(service_type)
                    .to_string(),
            ),
            None,
        ),
        _ => (VpnKind::Other("VPN".into()), None),
    }
}

fn global_dns(cfg: &GlobalDnsConfiguration) -> GlobalDns {
    let mut domains: Vec<(String, Vec<String>)> = cfg
        .domains
        .iter()
        .filter(|(name, _)| name.as_str() != "*")
        .map(|(name, domain)| (name.clone(), domain.servers.clone()))
        .collect();
    domains.sort();
    GlobalDns {
        searches: cfg.searches.clone(),
        servers: cfg.default_servers().to_vec(),
        domains,
    }
}

async fn load_profile(nm: &NetworkManager, uuid: &str) -> Result<ProfileDetail> {
    let saved = nm.get_saved_connection(uuid).await?;
    let raw = nm.get_saved_connection_raw(uuid).await?;
    let active = nm
        .list_active_connections()
        .await
        .unwrap_or_default()
        .iter()
        .any(|c| active_uuid(c) == uuid);

    let mac_randomization = match &saved.summary {
        SettingsSummary::Wifi {
            mac_randomization, ..
        } => mac_randomization.clone(),
        _ => None,
    };
    let metered = raw
        .get("connection")
        .and_then(|c| c.get("metered"))
        .and_then(|v| i32::try_from(v).ok())
        .and_then(|m| match m {
            1 | 3 => Some(true),
            2 | 4 => Some(false),
            _ => None,
        });

    let mut sections: Vec<(String, Vec<(String, String)>)> = raw
        .iter()
        .map(|(name, values)| {
            let mut entries: Vec<(String, String)> = values
                .iter()
                .map(|(k, v)| (k.clone(), render_value(v)))
                .collect();
            entries.sort();
            (name.clone(), entries)
        })
        .collect();
    sections.sort_by(|a, b| {
        section_rank(&a.0)
            .cmp(&section_rank(&b.0))
            .then(a.0.cmp(&b.0))
    });

    Ok(ProfileDetail {
        profile: Profile {
            uuid: saved.uuid.clone(),
            name: saved.id.clone(),
            kind: ProfileKind::from_nm(&saved.connection_type),
            interface: saved.interface_name.clone(),
            autoconnect: saved.autoconnect,
            priority: saved.autoconnect_priority,
            last_used: saved.timestamp_unix,
            active,
        },
        ipv4: saved.ipv4.as_ref().map(ip_config).unwrap_or_default(),
        ipv6: saved.ipv6.as_ref().map(ip_config).unwrap_or_default(),
        mac_randomization,
        metered,
        sections,
    })
}

fn section_rank(name: &str) -> u8 {
    match name {
        "connection" => 0,
        "ipv4" => 2,
        "ipv6" => 3,
        "proxy" => 4,
        _ => 1,
    }
}

fn render_value(value: &OwnedValue) -> String {
    match &**value {
        Value::Str(s) => s.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::U32(n) => n.to_string(),
        Value::I32(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::I64(n) => n.to_string(),
        Value::Array(array) if array.element_signature() == "y" => {
            let bytes: Vec<u8> = array
                .iter()
                .filter_map(|v| u8::try_from(v.clone()).ok())
                .collect();
            match std::str::from_utf8(&bytes) {
                Ok(s) if s.chars().all(|c| !c.is_control()) => s.to_string(),
                _ => bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(":"),
            }
        }
        other => other.to_string(),
    }
}

fn ip_config<A: std::fmt::Display>(ip: &IpSettings<A>) -> IpConfig {
    IpConfig {
        method: match ip.method {
            NmIpMethod::Manual => IpMethod::Manual,
            NmIpMethod::LinkLocal => IpMethod::LinkLocal,
            NmIpMethod::Shared => IpMethod::Shared,
            NmIpMethod::Disabled | NmIpMethod::Ignore => IpMethod::Disabled,
            _ => IpMethod::Auto,
        },
        addresses: ip.addresses.iter().map(|a| a.to_string()).collect(),
        gateway: ip.gateway.as_ref().map(|g| g.to_string()),
        dns: ip.dns.iter().map(|d| d.to_string()).collect(),
        search: ip.dns_search.clone(),
        ignore_auto_dns: ip.ignore_auto_dns,
        never_default: ip.never_default,
    }
}

async fn execute(nm: &NetworkManager, action: Action) -> Result<()> {
    match action {
        Action::Scan => nm.scan_networks(None).await?,
        Action::SetWifi(on) => nm.set_wireless_enabled(on).await?,
        Action::SetWwan(on) => nm.set_wwan_enabled(on).await?,
        Action::SetBluetooth(on) => nm.set_bluetooth_radio_enabled(on).await?,
        Action::SetAirplane(on) => nm.set_airplane_mode(on).await?,
        Action::Connect {
            ssid,
            interface,
            bssid,
            security,
            credentials,
            hidden,
        } => {
            let creds = wifi_security(security, credentials)?;
            if hidden {
                let builder = WifiConnectionBuilder::new(&ssid).hidden(true);
                let builder = match creds {
                    WifiSecurity::Open => builder.open(),
                    WifiSecurity::WpaPsk { psk } => builder.wpa_psk(psk),
                    WifiSecurity::Sae { psk } => builder.sae(psk),
                    WifiSecurity::WpaEap { opts } => builder.try_wpa_eap(opts)?,
                    _ => return Err(anyhow!("unsupported security for hidden network")),
                };
                nm.add_and_activate_connection(builder.build(), interface.as_deref(), None)
                    .await?;
            } else {
                nm.connect_to_bssid(&ssid, bssid.as_deref(), interface.as_deref(), creds)
                    .await?;
            }
        }
        Action::Activate(uuid) => {
            if is_vpn(nm, &uuid).await {
                nm.connect_vpn_by_uuid(&uuid).await?
            } else {
                nm.connect_by_uuid(&uuid, ConnectByUuidConfig::default())
                    .await?
            }
        }
        Action::Deactivate(uuid) => {
            if is_vpn(nm, &uuid).await {
                nm.disconnect_vpn_by_uuid(&uuid).await?
            } else {
                nm.disconnect_by_uuid(&uuid).await?
            }
        }
        Action::DisconnectDevice(interface) => nm.disconnect(Some(&interface)).await?,
        Action::DeleteProfile(uuid) => nm.delete_saved_connection(&uuid).await?,
        Action::UpdateProfile(uuid, edit) => {
            nm.update_saved_connection(&uuid, settings_patch(&edit)?)
                .await?
        }
        Action::ConnectBluetooth { address, name, dun } => {
            let role = if dun {
                BluetoothNetworkRole::Dun
            } else {
                BluetoothNetworkRole::PanU
            };
            let identity = BluetoothIdentity::new(address, role)?;
            nm.connect_bluetooth(&name, &identity).await?
        }
        Action::ImportOvpn {
            path,
            user,
            password,
        } => {
            nm.import_ovpn(&path, user.as_deref(), password.as_deref())
                .await?
        }
        Action::AddWireGuard(form) => {
            let mut peer = WireGuardPeer::new(
                form.peer_public_key.clone(),
                form.endpoint.clone(),
                form.allowed_ips.clone(),
            );
            if let Some(psk) = &form.preshared_key {
                peer = peer.with_preshared_key(psk.clone());
            }
            if let Some(keepalive) = form.keepalive {
                peer = peer.with_persistent_keepalive(keepalive);
            }
            let mut config = WireGuardConfig::new(
                form.name.clone(),
                form.endpoint.clone(),
                form.private_key.clone(),
                form.address.clone(),
                vec![peer],
            );
            if !form.dns.is_empty() {
                config = config.with_dns(form.dns.clone());
            }
            if let Some(mtu) = form.mtu {
                config = config.with_mtu(mtu);
            }
            nm.connect_vpn(config).await?
        }
        Action::StartHotspot(form) => {
            let settings = WifiConnectionBuilder::new(&form.ssid)
                .mode(WifiMode::Ap)
                .wpa_psk(form.password.clone())
                .ipv4_shared()
                .autoconnect(false)
                .build();
            nm.add_and_activate_connection(settings, form.interface.as_deref(), None)
                .await?;
        }
        Action::SetManaged(interface, on) => nm.set_device_managed(&interface, on).await?,
        Action::SetDeviceAutoconnect(interface, on) => {
            nm.set_device_autoconnect(&interface, on).await?
        }
        Action::SetGlobalDns(dns) => {
            let mut config = GlobalDnsConfiguration::new()
                .with_searches(dns.searches.clone())
                .with_default_servers(dns.servers.clone());
            for (name, servers) in &dns.domains {
                config = config.with_domain(
                    name.clone(),
                    GlobalDnsDomain::new().with_servers(servers.clone()),
                );
            }
            nm.set_global_dns_configuration(&config).await?
        }
        Action::CheckConnectivity => {
            nm.check_connectivity().await?;
        }
    }
    Ok(())
}

async fn is_vpn(nm: &NetworkManager, uuid: &str) -> bool {
    nm.list_vpn_connections()
        .await
        .map(|vpns| vpns.iter().any(|v| v.uuid == uuid))
        .unwrap_or(false)
}

fn wifi_security(security: Security, credentials: Credentials) -> Result<WifiSecurity> {
    Ok(match (security, credentials) {
        (Security::Sae, Credentials::Password(psk)) => WifiSecurity::Sae { psk },
        (_, Credentials::Password(psk)) => WifiSecurity::WpaPsk { psk },
        (_, Credentials::Enterprise(eap)) => WifiSecurity::WpaEap {
            opts: eap_options(&eap),
        },
        (Security::Open | Security::Owe, Credentials::None) => WifiSecurity::Open,
        (_, Credentials::None) => return Err(anyhow!("a password is required")),
    })
}

fn eap_options(eap: &Enterprise) -> EapOptions {
    let mut opts = match eap.method {
        EapMethod::Tls => EapOptions::new_tls_path(
            eap.identity.clone(),
            eap.private_key.clone().unwrap_or_default(),
            eap.client_cert.clone().unwrap_or_default(),
        ),
        method => EapOptions::new(eap.identity.clone(), eap.password.clone())
            .with_method(match method {
                EapMethod::Ttls => nmrs::EapMethod::Ttls,
                _ => nmrs::EapMethod::Peap,
            })
            .with_phase2(match eap.inner {
                InnerAuth::Pap => nmrs::Phase2::Pap,
                InnerAuth::Mschapv2 => nmrs::Phase2::Mschapv2,
            }),
    };
    if let Some(anon) = &eap.anonymous_identity {
        opts = opts.with_anonymous_identity(anon.clone());
    }
    if let Some(domain) = &eap.domain {
        opts = opts.with_domain_suffix_match(domain.clone());
    }
    if let Some(ca) = &eap.ca_cert {
        opts = opts.with_ca_cert_path(ca.clone());
    }
    if let Some(pw) = &eap.private_key_password {
        opts = opts.with_private_key_password(pw.clone());
    }
    opts.with_system_ca_certs(eap.system_ca)
}

fn settings_patch(edit: &ProfileEdit) -> Result<SettingsPatch> {
    let mut patch = SettingsPatch::default();
    patch.id = Some(edit.name.clone());
    patch.autoconnect = Some(edit.autoconnect);
    patch.autoconnect_priority = Some(edit.priority);
    patch.interface_name = Some(edit.interface.clone().filter(|s| !s.is_empty()));

    let mut overlay = HashMap::new();
    overlay.insert("ipv4".to_string(), ip_section(&edit.ipv4, false)?);
    overlay.insert("ipv6".to_string(), ip_section(&edit.ipv6, true)?);
    patch.raw_overlay = Some(overlay);
    Ok(patch)
}

fn owned<'a>(value: impl Into<Value<'a>>) -> Result<OwnedValue> {
    Ok(OwnedValue::try_from(value.into())?)
}

fn ip_section(ip: &IpConfig, v6: bool) -> Result<HashMap<String, OwnedValue>> {
    let mut section = HashMap::new();
    let method = match (ip.method, v6) {
        (IpMethod::Auto, _) => "auto",
        (IpMethod::Manual, _) => "manual",
        (IpMethod::LinkLocal, _) => "link-local",
        (IpMethod::Shared, _) => "shared",
        (IpMethod::Disabled, false) => "disabled",
        (IpMethod::Disabled, true) => "ignore",
    };
    section.insert("method".into(), owned(Str::from(method))?);

    let addresses: Vec<HashMap<String, Value<'static>>> = ip
        .addresses
        .iter()
        .filter_map(|cidr| {
            let (addr, prefix) = cidr
                .split_once('/')
                .unwrap_or((cidr, if v6 { "64" } else { "24" }));
            let prefix: u32 = prefix.trim().parse().ok()?;
            let mut entry = HashMap::new();
            entry.insert("address".to_string(), Value::from(addr.trim().to_string()));
            entry.insert("prefix".to_string(), Value::from(prefix));
            Some(entry)
        })
        .collect();
    section.insert("address-data".into(), owned(addresses)?);

    if let Some(gateway) = ip.gateway.as_ref().filter(|g| !g.is_empty()) {
        section.insert("gateway".into(), owned(Str::from(gateway.clone()))?);
    }

    if v6 {
        let dns: Vec<Vec<u8>> = ip
            .dns
            .iter()
            .filter_map(|s| s.trim().parse::<Ipv6Addr>().ok())
            .map(|a| a.octets().to_vec())
            .collect();
        section.insert("dns".into(), owned(dns)?);
    } else {
        let dns: Vec<u32> = ip
            .dns
            .iter()
            .filter_map(|s| s.trim().parse::<Ipv4Addr>().ok())
            .map(|a| u32::from_ne_bytes(a.octets()))
            .collect();
        section.insert("dns".into(), owned(dns)?);
    }
    section.insert("dns-search".into(), owned(ip.search.clone())?);
    section.insert("ignore-auto-dns".into(), owned(ip.ignore_auto_dns)?);
    section.insert("never-default".into(), owned(ip.never_default)?);
    Ok(section)
}
