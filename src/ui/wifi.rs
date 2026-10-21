use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Icon, StyledExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::{NmApp, ghost, input, set, value};
use crate::config::{SignalStyle, Sort};
use crate::model::Action;
use crate::model::*;
use crate::ui::kit;

pub struct EapInputs {
    identity: Entity<InputState>,
    password: Entity<InputState>,
    anonymous: Entity<InputState>,
    domain: Entity<InputState>,
    ca: Entity<InputState>,
    cert: Entity<InputState>,
    key: Entity<InputState>,
    key_password: Entity<InputState>,
}

pub struct WifiUi {
    pub expanded: Option<String>,
    pub bssid: Option<String>,
    pub password: Entity<InputState>,
    eap: EapInputs,
    eap_method: EapMethod,
    eap_inner: InnerAuth,
    eap_system_ca: bool,
    pub hidden_open: bool,
    hidden_ssid: Entity<InputState>,
    hidden_password: Entity<InputState>,
    hidden_security: Security,
    pub hotspot_open: bool,
    hotspot_ssid: Entity<InputState>,
    hotspot_password: Entity<InputState>,
    pub interface: Option<String>,
}

impl WifiUi {
    pub fn new(window: &mut Window, cx: &mut Context<NmApp>) -> Self {
        Self {
            expanded: None,
            bssid: None,
            password: input("", true, window, cx),
            eap: EapInputs {
                identity: input("user@example.org", false, window, cx),
                password: input("", true, window, cx),
                anonymous: input("Optional", false, window, cx),
                domain: input("Optional", false, window, cx),
                ca: input("/path/to/ca.pem", false, window, cx),
                cert: input("/path/to/client.pem", false, window, cx),
                key: input("/path/to/client.key", false, window, cx),
                key_password: input("Optional", true, window, cx),
            },
            eap_method: EapMethod::Peap,
            eap_inner: InnerAuth::Mschapv2,
            eap_system_ca: true,
            hidden_open: false,
            hidden_ssid: input("", false, window, cx),
            hidden_password: input("", true, window, cx),
            hidden_security: Security::Psk,
            hotspot_open: false,
            hotspot_ssid: input("", false, window, cx),
            hotspot_password: input("At least 8 characters", true, window, cx),
            interface: None,
        }
    }
}

pub fn subscribe(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let enter = |input: &Entity<InputState>,
                 window: &mut Window,
                 cx: &mut Context<NmApp>,
                 f: fn(&mut NmApp, &mut Window, &mut Context<NmApp>)| {
        cx.subscribe_in(
            input,
            window,
            move |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    f(this, window, cx);
                }
            },
        )
    };
    let subs = [
        enter(&app.wifi.password.clone(), window, cx, connect_expanded),
        enter(&app.wifi.eap.password.clone(), window, cx, connect_expanded),
        enter(
            &app.wifi.hidden_password.clone(),
            window,
            cx,
            connect_hidden,
        ),
        enter(&app.wifi.hidden_ssid.clone(), window, cx, connect_hidden),
        enter(
            &app.wifi.hotspot_password.clone(),
            window,
            cx,
            start_hotspot,
        ),
    ];
    app.subscriptions.extend(subs);
}

pub fn dismiss(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) -> bool {
    let ui = &mut app.wifi;
    if ui.expanded.is_some() {
        ui.expanded = None;
        ui.bssid = None;
        let password = ui.password.clone();
        set(&password, "", window, cx);
        return true;
    }
    if ui.hidden_open || ui.hotspot_open {
        ui.hidden_open = false;
        ui.hotspot_open = false;
        return true;
    }
    false
}

fn toggle(app: &mut NmApp, ssid: &str, window: &mut Window, cx: &mut Context<NmApp>) {
    if app.wifi.expanded.as_deref() == Some(ssid) {
        app.wifi.expanded = None;
    } else {
        app.wifi.expanded = Some(ssid.to_string());
        app.wifi.bssid = None;
        let password = app.wifi.password.clone();
        set(&password, "", window, cx);
        let needs_password = app
            .state
            .as_ref()
            .and_then(|s| s.networks.iter().find(|n| n.ssid == ssid))
            .is_some_and(|n| !n.known && n.security.needs_secret());
        if needs_password {
            password.update(cx, |state, cx| state.focus(window, cx));
        }
    }
    app.confirming = None;
    cx.notify();
}

fn enterprise(app: &NmApp, cx: &App) -> Enterprise {
    let e = &app.wifi.eap;
    let opt = |i: &Entity<InputState>| Some(value(i, cx)).filter(|s| !s.is_empty());
    Enterprise {
        method: app.wifi.eap_method,
        inner: app.wifi.eap_inner,
        identity: value(&e.identity, cx),
        password: e.password.read(cx).value().to_string(),
        anonymous_identity: opt(&e.anonymous),
        domain: opt(&e.domain),
        ca_cert: opt(&e.ca),
        system_ca: app.wifi.eap_system_ca,
        client_cert: opt(&e.cert),
        private_key: opt(&e.key),
        private_key_password: opt(&e.key_password),
    }
}

fn connect_expanded(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    if let Some(ssid) = app.wifi.expanded.clone() {
        connect(app, &ssid, window, cx);
    }
}

fn connect(app: &mut NmApp, ssid: &str, window: &mut Window, cx: &mut Context<NmApp>) {
    let Some(net) = app
        .state
        .as_ref()
        .and_then(|s| s.networks.iter().find(|n| n.ssid == ssid))
        .cloned()
    else {
        return;
    };
    let key = format!("wifi:{ssid}");
    let pinned = app.wifi.bssid.is_some();
    if let (true, Some(uuid)) = (net.known, net.profile.clone())
        && (!pinned || net.security == Security::Eap)
    {
        return after_connect(app, key, Action::Activate(uuid), window, cx);
    }
    let credentials = match net.security {
        _ if net.known && net.security.needs_secret() => Credentials::Password(String::new()),
        Security::Open | Security::Owe => Credentials::None,
        Security::Eap => Credentials::Enterprise(Box::new(enterprise(app, cx))),
        _ => {
            let password = app.wifi.password.read(cx).value().to_string();
            if password.is_empty() {
                let input = app.wifi.password.clone();
                input.update(cx, |state, cx| state.focus(window, cx));
                return;
            }
            Credentials::Password(password)
        }
    };
    let action = Action::Connect {
        ssid: net.ssid.clone(),
        interface: app.wifi.interface.clone(),
        bssid: app.wifi.bssid.clone(),
        security: net.security,
        credentials,
        hidden: false,
    };
    after_connect(app, key, action, window, cx);
}

fn after_connect(
    app: &mut NmApp,
    key: String,
    action: Action,
    window: &mut Window,
    cx: &mut Context<NmApp>,
) {
    app.run_then(key, action, window, cx, |this, window, cx| {
        this.wifi.expanded = None;
        this.wifi.bssid = None;
        let password = this.wifi.password.clone();
        set(&password, "", window, cx);
        if this.config.behavior.close_on_connect {
            cx.quit();
        }
    });
}

fn connect_hidden(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let ssid = value(&app.wifi.hidden_ssid, cx);
    if ssid.is_empty() {
        return;
    }
    let security = app.wifi.hidden_security;
    let credentials = match security {
        Security::Open => Credentials::None,
        Security::Eap => Credentials::Enterprise(Box::new(enterprise(app, cx))),
        _ => Credentials::Password(app.wifi.hidden_password.read(cx).value().to_string()),
    };
    let action = Action::Connect {
        ssid,
        interface: app.wifi.interface.clone(),
        bssid: None,
        security,
        credentials,
        hidden: true,
    };
    app.run_then("wifi:hidden", action, window, cx, |this, window, cx| {
        this.wifi.hidden_open = false;
        let (a, b) = (
            this.wifi.hidden_ssid.clone(),
            this.wifi.hidden_password.clone(),
        );
        set(&a, "", window, cx);
        set(&b, "", window, cx);
    });
}

fn start_hotspot(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let ssid = value(&app.wifi.hotspot_ssid, cx);
    let password = app.wifi.hotspot_password.read(cx).value().to_string();
    if ssid.is_empty() || password.len() < 8 {
        return;
    }
    let action = Action::StartHotspot(HotspotForm {
        ssid,
        password,
        interface: app.wifi.interface.clone(),
    });
    app.run_then("wifi:hotspot", action, window, cx, |this, _, _| {
        this.wifi.hotspot_open = false;
    });
}

fn subtitle(app: &NmApp, net: &WifiNetwork) -> String {
    let cfg = &app.config.wifi;
    let mut parts: Vec<String> = Vec::new();
    if net.active {
        parts.push("Connected".into());
    } else if net.known {
        parts.push("Saved".into());
    }
    if cfg.show_security {
        parts.push(net.security.label().into());
    }
    if cfg.show_band
        && let Some(band) = net.band()
    {
        parts.push(band.into());
    }
    if cfg.show_bssid
        && let Some(ap) = net.access_points.iter().max_by_key(|a| a.strength)
    {
        parts.push(ap.bssid.clone());
    }
    parts.join(" · ")
}

fn signal(app: &NmApp, net: &WifiNetwork, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let pct = matches!(
        app.config.wifi.signal,
        SignalStyle::Percent | SignalStyle::Both
    );
    div()
        .h_flex()
        .items_center()
        .gap_2()
        .flex_shrink_0()
        .text_xs()
        .text_color(theme.muted_foreground)
        .when(net.security.is_secured(), |s| {
            s.child(Icon::new(IconName::Lock).size_3())
        })
        .when(pct, |s| {
            s.child(div().w_8().text_right().child(format!("{}%", net.strength)))
        })
        .into_any_element()
}

fn network_row(app: &NmApp, net: &WifiNetwork, index: usize, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let ssid = net.ssid.clone();
    let expanded = app.wifi.expanded.as_deref() == Some(net.ssid.as_str());
    let pending = app.is_pending(&format!("wifi:{}", net.ssid));
    let show_icon = app.config.wifi.signal != SignalStyle::Percent;
    let leading = if net.active {
        kit::badge_icon(kit::signal_icon(net.strength), true, cx).into_any_element()
    } else if !show_icon {
        div().into_any_element()
    } else {
        div()
            .size_8()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .text_color(if net.known {
                theme.foreground
            } else {
                theme.muted_foreground
            })
            .child(Icon::new(kit::signal_icon(net.strength)).size_4())
            .into_any_element()
    };
    let row = kit::row(
        SharedString::from(format!("net-{}", net.ssid)),
        app.row_height(),
        cx,
    )
    .cursor_pointer()
    .when(expanded, |s| s.bg(theme.accent.opacity(0.35)))
    .on_click(cx.listener(move |this, _, window, cx| toggle(this, &ssid, window, cx)))
    .child(leading)
    .child(kit::title_block(
        net.ssid.clone(),
        Some(subtitle(app, net)),
        cx,
    ))
    .when(pending, |s| {
        s.child(
            Icon::new(IconName::LoaderCircle)
                .size_4()
                .text_color(theme.muted_foreground)
                .with_animation(
                    SharedString::from(format!("spin-{}", net.ssid)),
                    Animation::new(std::time::Duration::from_secs(1)).repeat(),
                    |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
                ),
        )
    })
    .child(signal(app, net, cx));

    div()
        .v_flex()
        .map(kit::divided(index, cx))
        .child(row)
        .when(expanded, |s| s.child(expanded_panel(app, net, cx)))
}

fn access_points(app: &NmApp, net: &WifiNetwork, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let selected = app.wifi.bssid.clone();
    let mut aps = net.access_points.clone();
    aps.sort_by_key(|a| std::cmp::Reverse(a.strength));
    div()
        .v_flex()
        .px_3p5()
        .pt_3()
        .pb_2()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .pb_1p5()
                .child("Access points"),
        )
        .children(aps.into_iter().map(|ap| {
            let bssid = ap.bssid.clone();
            let picked = selected.as_deref() == Some(ap.bssid.as_str());
            div()
                .id(SharedString::from(format!("ap-{}", ap.bssid)))
                .h_flex()
                .items_center()
                .gap_3()
                .h_8()
                .px_2()
                .rounded(theme.radius)
                .cursor_pointer()
                .text_xs()
                .when(picked, |s| s.bg(theme.primary.opacity(0.12)))
                .hover(|s| s.bg(theme.accent))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.wifi.bssid = if this.wifi.bssid.as_deref() == Some(bssid.as_str()) {
                        None
                    } else {
                        Some(bssid.clone())
                    };
                    cx.notify();
                }))
                .child(
                    Icon::new(if picked {
                        IconName::CircleDot
                    } else {
                        kit::signal_icon(ap.strength)
                    })
                    .size_3p5()
                    .text_color(if picked {
                        theme.primary
                    } else {
                        theme.muted_foreground
                    }),
                )
                .child(
                    div()
                        .flex_1()
                        .font_family(theme.mono_font_family.clone())
                        .child(ap.bssid.clone()),
                )
                .when(ap.active, |s| s.child(kit::accent_tag("In use", cx)))
                .child(
                    div()
                        .w_16()
                        .text_color(theme.muted_foreground)
                        .child(band(ap.frequency).unwrap_or("—")),
                )
                .child(
                    div()
                        .w_10()
                        .text_right()
                        .text_color(theme.muted_foreground)
                        .child(format!("{}%", ap.strength)),
                )
        }))
}

fn expanded_panel(app: &NmApp, net: &WifiNetwork, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let ssid = net.ssid.clone();
    let pending = app.is_pending(&format!("wifi:{}", net.ssid));
    let mut panel = div()
        .v_flex()
        .bg(theme.background.opacity(0.35))
        .border_t_1()
        .border_color(theme.border);

    if net.active || net.known {
        let mut items = Vec::new();
        if let Some(ip) = &net.ip4 {
            items.push(("IPv4", ip.clone()));
        }
        if let Some(ip) = &net.ip6 {
            items.push(("IPv6", ip.clone()));
        }
        items.push(("Security", net.security.label().to_string()));
        if let (Some(b), Some(ch)) = (net.band(), net.channel()) {
            items.push(("Band", format!("{b} · Channel {ch}")));
        }
        if let Some(ap) = net.access_points.iter().find(|a| a.active) {
            items.push(("Link rate", format!("{} Mb/s", ap.rate_mbps)));
        }
        items.push(("Signal", format!("{}%", net.strength)));
        panel = panel.child(kit::kv_grid(items, cx));
    }

    if net.access_points.len() > 1 || app.config.wifi.show_bssid {
        panel = panel.child(access_points(app, net, cx));
    }

    let needs_form = !net.known && !net.active;
    if needs_form && net.security.needs_secret() {
        panel =
            panel.child(kit::form(cx).child(kit::secret_field("Password", &app.wifi.password, cx)));
    }
    if needs_form && net.security == Security::Eap {
        panel = panel.child(enterprise_form(app, cx));
    }

    let forget_key = format!("forget:{}", net.ssid);
    let armed = app.is_confirming(&forget_key);
    let profile = net.profile.clone();
    let profile_edit = net.profile.clone();
    let iface = net.interface.clone();

    let actions = kit::actions()
        .px_3p5()
        .pb_3p5()
        .when(net.known, |s| {
            s.child(kit::danger("forget", "Forget", armed).on_click(cx.listener(
                move |this, _, window, cx| {
                    if let Some(uuid) = profile.clone()
                        && this.confirm(&forget_key, cx)
                    {
                        this.run(forget_key.clone(), Action::DeleteProfile(uuid), window, cx);
                    }
                },
            )))
            .child(kit::plain("edit", "Edit profile").on_click(cx.listener(
                move |this, _, window, cx| {
                    if let Some(uuid) = profile_edit.clone() {
                        crate::ui::profiles::open(this, uuid, window, cx);
                    }
                },
            )))
        })
        .child(div().flex_1())
        .map(|s| {
            if net.active {
                let profile = net.profile.clone();
                s.child(kit::plain("disconnect", "Disconnect").on_click(cx.listener(
                    move |this, _, window, cx| {
                        let action = match profile.clone() {
                            Some(uuid) => Action::Deactivate(uuid),
                            None => Action::DisconnectDevice(iface.clone()),
                        };
                        this.run(format!("wifi:{ssid}"), action, window, cx);
                    },
                )))
            } else {
                s.child(kit::primary("connect", "Connect", pending).on_click(
                    cx.listener(move |this, _, window, cx| connect(this, &ssid, window, cx)),
                ))
            }
        });
    panel.child(actions)
}

fn enterprise_form(app: &NmApp, cx: &mut Context<NmApp>) -> Div {
    let e = &app.wifi.eap;
    let method = app.wifi.eap_method;
    let tls = method == EapMethod::Tls;
    kit::form(cx)
        .child(
            div()
                .h_flex()
                .gap_3()
                .child(kit::segmented(
                    "eap-method",
                    &[
                        (EapMethod::Peap, "PEAP"),
                        (EapMethod::Ttls, "TTLS"),
                        (EapMethod::Tls, "TLS"),
                    ],
                    method,
                    cx,
                    |this, v, _, cx| {
                        this.wifi.eap_method = v;
                        cx.notify();
                    },
                ))
                .when(!tls, |s| {
                    s.child(kit::segmented(
                        "eap-inner",
                        &[(InnerAuth::Mschapv2, "MSCHAPv2"), (InnerAuth::Pap, "PAP")],
                        app.wifi.eap_inner,
                        cx,
                        |this, v, _, cx| {
                            this.wifi.eap_inner = v;
                            cx.notify();
                        },
                    ))
                }),
        )
        .child(kit::pair(
            kit::field("Identity", &e.identity, cx),
            if tls {
                kit::secret_field("Key password", &e.key_password, cx)
            } else {
                kit::secret_field("Password", &e.password, cx)
            },
        ))
        .when(tls, |s| {
            s.child(kit::pair(
                kit::field("Client certificate", &e.cert, cx),
                kit::field("Private key", &e.key, cx),
            ))
        })
        .child(kit::pair(
            kit::field("Anonymous identity", &e.anonymous, cx),
            kit::field("Domain", &e.domain, cx),
        ))
        .child(kit::pair(
            kit::field("CA certificate", &e.ca, cx),
            kit::labeled(
                "System CA store",
                div().h_8().flex().items_center().child(
                    Switch::new("eap-system-ca")
                        .checked(app.wifi.eap_system_ca)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.wifi.eap_system_ca = *checked;
                            cx.notify();
                        })),
                ),
                cx,
            ),
        ))
}

fn hidden_form(app: &NmApp, cx: &mut Context<NmApp>) -> Div {
    let security = app.wifi.hidden_security;
    let pending = app.is_pending("wifi:hidden");
    kit::card(cx).mb_4().child(
        kit::form(cx)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .font_medium()
                            .child("Hidden network"),
                    )
                    .child(kit::segmented(
                        "hidden-sec",
                        &[
                            (Security::Open, "Open"),
                            (Security::Psk, "WPA2"),
                            (Security::Sae, "WPA3"),
                            (Security::Eap, "Enterprise"),
                        ],
                        security,
                        cx,
                        |this, v, _, cx| {
                            this.wifi.hidden_security = v;
                            cx.notify();
                        },
                    )),
            )
            .child(match security {
                Security::Open | Security::Eap => {
                    kit::field("Network name", &app.wifi.hidden_ssid, cx)
                }
                _ => kit::pair(
                    kit::field("Network name", &app.wifi.hidden_ssid, cx),
                    kit::secret_field("Password", &app.wifi.hidden_password, cx),
                ),
            })
            .when(security == Security::Eap, |s| {
                s.child(enterprise_form(app, cx).p_0())
            })
            .child(
                kit::actions()
                    .child(
                        kit::plain("hidden-cancel", "Cancel")
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.wifi.hidden_open = false;
                                cx.notify();
                            })),
                    )
                    .child(kit::primary("hidden-connect", "Connect", pending).on_click(
                        cx.listener(|this, _, window, cx| connect_hidden(this, window, cx)),
                    )),
            ),
    )
}

fn hotspot_form(app: &NmApp, cx: &mut Context<NmApp>) -> Div {
    let pending = app.is_pending("wifi:hotspot");
    kit::card(cx).mb_4().child(
        kit::form(cx)
            .child(div().text_sm().font_medium().child("Hotspot"))
            .child(kit::pair(
                kit::field("Network name", &app.wifi.hotspot_ssid, cx),
                kit::secret_field("Password", &app.wifi.hotspot_password, cx),
            ))
            .child(
                kit::actions()
                    .child(
                        kit::plain("hotspot-cancel", "Cancel")
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.wifi.hotspot_open = false;
                                cx.notify();
                            })),
                    )
                    .child(kit::primary("hotspot-start", "Start", pending).on_click(
                        cx.listener(|this, _, window, cx| start_hotspot(this, window, cx)),
                    )),
            ),
    )
}

pub fn render(app: &mut NmApp, _window: &mut Window, cx: &mut Context<NmApp>) -> AnyElement {
    let Some(state) = app.state.clone() else {
        return div().into_any_element();
    };
    let radio = state.radios.wifi;
    let scanning = app.is_pending("scan");
    let toggling = app.is_pending("radio:wifi");

    let mut header = kit::header("Wi-Fi", cx);
    if state.wifi_devices.len() > 1 {
        let current = app
            .wifi
            .interface
            .clone()
            .unwrap_or_else(|| state.wifi_devices[0].interface.clone());
        let options: Vec<(usize, SharedString)> = state
            .wifi_devices
            .iter()
            .enumerate()
            .map(|(i, d)| (i, SharedString::from(d.interface.clone())))
            .collect();
        let selected = state
            .wifi_devices
            .iter()
            .position(|d| d.interface == current)
            .unwrap_or(0);
        let ifaces: Vec<String> = state
            .wifi_devices
            .iter()
            .map(|d| d.interface.clone())
            .collect();
        header = header.child(kit::segmented(
            "wifi-dev",
            &options,
            selected,
            cx,
            move |this, i, _, cx| {
                this.wifi.interface = ifaces.get(i).cloned();
                cx.notify();
            },
        ));
    }
    if radio.enabled && radio.usable() {
        header = header
            .child(
                ghost("scan", IconName::RefreshCw, "Scan")
                    .loading(scanning)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.run("scan", Action::Scan, window, cx)
                    })),
            )
            .child(
                ghost("hidden", IconName::WifiPen, "Hidden network").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.wifi.hidden_open = !this.wifi.hidden_open;
                        this.wifi.hotspot_open = false;
                        cx.notify();
                    },
                )),
            )
            .child(
                ghost("hotspot", IconName::RadioTower, "Hotspot").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.wifi.hotspot_open = !this.wifi.hotspot_open;
                        this.wifi.hidden_open = false;
                        cx.notify();
                    },
                )),
            );
    }
    if radio.present {
        header = header.child(
            Switch::new("wifi-radio")
                .checked(radio.enabled)
                .disabled(!radio.hardware || toggling)
                .on_click(cx.listener(|this, on: &bool, window, cx| {
                    this.run("radio:wifi", Action::SetWifi(*on), window, cx)
                })),
        );
    }

    let mut body = div().v_flex().child(header);

    if !radio.present || state.wifi_devices.is_empty() {
        return body
            .child(kit::empty(IconName::WifiOff, "No Wi-Fi adapter", None, cx))
            .into_any_element();
    }
    if !radio.hardware {
        return body
            .child(kit::empty(
                IconName::WifiOff,
                "Wi-Fi is disabled by a hardware switch",
                None,
                cx,
            ))
            .into_any_element();
    }
    if !radio.enabled {
        return body
            .child(kit::empty(
                IconName::WifiOff,
                "Wi-Fi is off",
                Some(
                    kit::primary("wifi-on", "Turn on", toggling).on_click(cx.listener(
                        |this, _, window, cx| {
                            this.run("radio:wifi", Action::SetWifi(true), window, cx)
                        },
                    )),
                ),
                cx,
            ))
            .into_any_element();
    }

    if app.wifi.hidden_open {
        body = body.child(hidden_form(app, cx));
    }
    if app.wifi.hotspot_open {
        body = body.child(hotspot_form(app, cx));
    }

    let iface = app.wifi.interface.clone();
    let cfg = app.config.wifi.clone();
    let mut nets: Vec<WifiNetwork> = state
        .networks
        .iter()
        .filter(|n| iface.as_ref().is_none_or(|i| &n.interface == i))
        .filter(|n| n.active || n.strength >= cfg.min_strength)
        .cloned()
        .collect();
    match cfg.sort {
        Sort::Signal => nets.sort_by_key(|n| std::cmp::Reverse(n.strength)),
        Sort::Name => nets.sort_by_key(|n| n.ssid.to_lowercase()),
    }

    let active: Vec<&WifiNetwork> = nets.iter().filter(|n| n.active).collect();
    let known: Vec<&WifiNetwork> = nets.iter().filter(|n| n.known && !n.active).collect();
    let other: Vec<&WifiNetwork> = nets.iter().filter(|n| !n.known && !n.active).collect();

    if nets.is_empty() {
        return body
            .child(kit::empty(
                IconName::WifiZero,
                "No networks in range",
                None,
                cx,
            ))
            .into_any_element();
    }

    if !active.is_empty() {
        body = body.child(
            kit::card(cx).children(
                active
                    .iter()
                    .enumerate()
                    .map(|(i, n)| network_row(app, n, i, cx)),
            ),
        );
    }
    if !known.is_empty() {
        body = body.child(kit::section("Saved", cx)).child(
            kit::card(cx).children(
                known
                    .iter()
                    .enumerate()
                    .map(|(i, n)| network_row(app, n, i, cx)),
            ),
        );
    }
    if !other.is_empty() {
        body = body.child(kit::section("Nearby", cx)).child(
            kit::card(cx).children(
                other
                    .iter()
                    .enumerate()
                    .map(|(i, n)| network_row(app, n, i, cx)),
            ),
        );
    }
    body.into_any_element()
}
