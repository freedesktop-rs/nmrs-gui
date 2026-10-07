use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::InputState;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Sizable, StyledExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::{NmApp, ghost, input, list, set, value};
use crate::model::Action;
use crate::model::*;
use crate::ui::kit;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Form {
    None,
    Import,
    WireGuard,
}

pub struct VpnUi {
    pub expanded: Option<String>,
    pub form: Form,
    ovpn_path: Entity<InputState>,
    ovpn_user: Entity<InputState>,
    ovpn_password: Entity<InputState>,
    wg_name: Entity<InputState>,
    wg_key: Entity<InputState>,
    wg_address: Entity<InputState>,
    wg_dns: Entity<InputState>,
    wg_mtu: Entity<InputState>,
    wg_peer: Entity<InputState>,
    wg_endpoint: Entity<InputState>,
    wg_allowed: Entity<InputState>,
    wg_psk: Entity<InputState>,
    wg_keepalive: Entity<InputState>,
}

impl VpnUi {
    pub fn new(window: &mut Window, cx: &mut Context<NmApp>) -> Self {
        Self {
            expanded: None,
            form: Form::None,
            ovpn_path: input("~/Downloads/client.ovpn", false, window, cx),
            ovpn_user: input("Optional", false, window, cx),
            ovpn_password: input("Optional", true, window, cx),
            wg_name: input("wg0", false, window, cx),
            wg_key: input("Base64 private key", true, window, cx),
            wg_address: input("10.0.0.2/32", false, window, cx),
            wg_dns: input("1.1.1.1, 9.9.9.9", false, window, cx),
            wg_mtu: input("1420", false, window, cx),
            wg_peer: input("Base64 public key", false, window, cx),
            wg_endpoint: input("vpn.example.com:51820", false, window, cx),
            wg_allowed: input("0.0.0.0/0, ::/0", false, window, cx),
            wg_psk: input("Optional", true, window, cx),
            wg_keepalive: input("25", false, window, cx),
        }
    }
}

pub fn dismiss(app: &mut NmApp, cx: &mut Context<NmApp>) -> bool {
    if app.vpn.form != Form::None {
        app.vpn.form = Form::None;
        cx.notify();
        return true;
    }
    if app.vpn.expanded.take().is_some() {
        cx.notify();
        return true;
    }
    false
}

fn expand_home(path: &str) -> String {
    match path.strip_prefix("~/") {
        Some(rest) => dirs::home_dir()
            .map(|h| h.join(rest).to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string()),
        None => path.to_string(),
    }
}

fn import(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let path = value(&app.vpn.ovpn_path, cx);
    if path.is_empty() {
        return;
    }
    let opt = |s: String| Some(s).filter(|s| !s.is_empty());
    let action = Action::ImportOvpn {
        path: expand_home(&path),
        user: opt(value(&app.vpn.ovpn_user, cx)),
        password: opt(app.vpn.ovpn_password.read(cx).value().to_string()),
    };
    app.run_then("vpn:import", action, window, cx, |this, window, cx| {
        this.vpn.form = Form::None;
        for input in [
            this.vpn.ovpn_path.clone(),
            this.vpn.ovpn_user.clone(),
            this.vpn.ovpn_password.clone(),
        ] {
            set(&input, "", window, cx);
        }
    });
}

fn add_wireguard(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let ui = &app.vpn;
    let form = WireGuardForm {
        name: value(&ui.wg_name, cx),
        private_key: value(&ui.wg_key, cx),
        address: value(&ui.wg_address, cx),
        dns: list(&ui.wg_dns, cx),
        mtu: value(&ui.wg_mtu, cx).parse().ok(),
        peer_public_key: value(&ui.wg_peer, cx),
        endpoint: value(&ui.wg_endpoint, cx),
        allowed_ips: {
            let ips = list(&ui.wg_allowed, cx);
            if ips.is_empty() {
                vec!["0.0.0.0/0".into(), "::/0".into()]
            } else {
                ips
            }
        },
        preshared_key: Some(value(&ui.wg_psk, cx)).filter(|s| !s.is_empty()),
        keepalive: value(&ui.wg_keepalive, cx).parse().ok(),
    };
    if form.name.is_empty()
        || form.private_key.is_empty()
        || form.address.is_empty()
        || form.peer_public_key.is_empty()
        || form.endpoint.is_empty()
    {
        return;
    }
    app.run_then(
        "vpn:wg",
        Action::AddWireGuard(Box::new(form)),
        window,
        cx,
        |this, window, cx| {
            this.vpn.form = Form::None;
            let ui = &this.vpn;
            for input in [
                ui.wg_name.clone(),
                ui.wg_key.clone(),
                ui.wg_address.clone(),
                ui.wg_dns.clone(),
                ui.wg_mtu.clone(),
                ui.wg_peer.clone(),
                ui.wg_endpoint.clone(),
                ui.wg_allowed.clone(),
                ui.wg_psk.clone(),
                ui.wg_keepalive.clone(),
            ] {
                set(&input, "", window, cx);
            }
        },
    );
}

fn import_form(app: &NmApp, cx: &mut Context<NmApp>) -> Div {
    let pending = app.is_pending("vpn:import");
    kit::card(cx).mb_4().child(
        kit::form(cx)
            .child(div().text_sm().font_medium().child("Import OpenVPN"))
            .child(kit::field("Configuration file", &app.vpn.ovpn_path, cx))
            .child(kit::pair(
                kit::field("Username", &app.vpn.ovpn_user, cx),
                kit::secret_field("Password", &app.vpn.ovpn_password, cx),
            ))
            .child(
                kit::actions()
                    .child(
                        kit::plain("imp-cancel", "Cancel")
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.vpn.form = Form::None;
                                cx.notify();
                            })),
                    )
                    .child(
                        kit::primary("imp-go", "Import", pending)
                            .on_click(cx.listener(|this, _, window, cx| import(this, window, cx))),
                    ),
            ),
    )
}

fn wireguard_form(app: &NmApp, cx: &mut Context<NmApp>) -> Div {
    let ui = &app.vpn;
    let pending = app.is_pending("vpn:wg");
    kit::card(cx).mb_4().child(
        kit::form(cx)
            .child(div().text_sm().font_medium().child("WireGuard"))
            .child(kit::pair(
                kit::field("Name", &ui.wg_name, cx),
                kit::field("Address", &ui.wg_address, cx),
            ))
            .child(kit::secret_field("Private key", &ui.wg_key, cx))
            .child(kit::pair(
                kit::field("DNS", &ui.wg_dns, cx),
                kit::field("MTU", &ui.wg_mtu, cx),
            ))
            .child(
                div()
                    .pt_2()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child("Peer"),
            )
            .child(kit::field("Public key", &ui.wg_peer, cx))
            .child(kit::pair(
                kit::field("Endpoint", &ui.wg_endpoint, cx),
                kit::field("Allowed IPs", &ui.wg_allowed, cx),
            ))
            .child(kit::pair(
                kit::secret_field("Preshared key", &ui.wg_psk, cx),
                kit::field("Keepalive", &ui.wg_keepalive, cx),
            ))
            .child(
                kit::actions()
                    .child(
                        kit::plain("wg-cancel", "Cancel")
                            .ghost()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.vpn.form = Form::None;
                                cx.notify();
                            })),
                    )
                    .child(kit::primary("wg-go", "Add and connect", pending).on_click(
                        cx.listener(|this, _, window, cx| add_wireguard(this, window, cx)),
                    )),
            ),
    )
}

fn vpn_row(app: &NmApp, vpn: &Vpn, index: usize, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let up = vpn.state.is_up();
    let busy = vpn.state.is_busy();
    let key = format!("vpn:{}", vpn.uuid);
    let pending = app.is_pending(&key);
    let expanded = app.vpn.expanded.as_deref() == Some(vpn.uuid.as_str());
    let uuid = vpn.uuid.clone();
    let uuid_toggle = vpn.uuid.clone();

    let mut subtitle = vec![vpn.kind.label().to_string()];
    if up {
        subtitle.insert(0, "Connected".into());
    } else if busy {
        subtitle.insert(0, vpn.state.label().into());
    }
    if let Some(gw) = &vpn.gateway {
        subtitle.push(gw.clone());
    }

    let row = kit::row(
        SharedString::from(format!("vpn-{}", vpn.uuid)),
        app.row_height(),
        cx,
    )
    .cursor_pointer()
    .when(expanded, |s| s.bg(theme.accent.opacity(0.35)))
    .on_click(cx.listener(move |this, _, _, cx| {
        this.vpn.expanded = if this.vpn.expanded.as_deref() == Some(uuid.as_str()) {
            None
        } else {
            Some(uuid.clone())
        };
        this.confirming = None;
        cx.notify();
    }))
    .child(kit::badge_icon(
        if up {
            IconName::ShieldCheck
        } else {
            IconName::Shield
        },
        up,
        cx,
    ))
    .child(kit::title_block(
        vpn.name.clone(),
        Some(subtitle.join(" · ")),
        cx,
    ))
    .child(
        div()
            .id(SharedString::from(format!("vpn-sw-wrap-{}", vpn.uuid)))
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                Switch::new(SharedString::from(format!("vpn-sw-{}", vpn.uuid)))
                    .checked(up || busy)
                    .disabled(pending)
                    .on_click(cx.listener(move |this, on: &bool, window, cx| {
                        let action = if *on {
                            Action::Activate(uuid_toggle.clone())
                        } else {
                            Action::Deactivate(uuid_toggle.clone())
                        };
                        this.run(format!("vpn:{uuid_toggle}"), action, window, cx);
                    })),
            ),
    );

    div()
        .v_flex()
        .map(kit::divided(index, cx))
        .child(row)
        .when(expanded, |s| s.child(vpn_details(app, vpn, cx)))
}

fn vpn_details(app: &NmApp, vpn: &Vpn, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let mut items = vec![("Type", vpn.kind.label().to_string())];
    if let Some(gw) = &vpn.gateway {
        items.push(("Gateway", gw.clone()));
    }
    if let Some(ip) = &vpn.ip4 {
        items.push(("Address", ip.clone()));
    }
    if let Some(iface) = &vpn.interface {
        items.push(("Interface", iface.clone()));
    }
    if let Some(user) = &vpn.user {
        items.push(("User", user.clone()));
    }
    items.push(("State", vpn.state.label().to_string()));

    let forget = format!("forget:{}", vpn.uuid);
    let armed = app.is_confirming(&forget);
    let uuid = vpn.uuid.clone();
    let uuid_edit = vpn.uuid.clone();
    let uuid_auto = vpn.uuid.clone();
    div()
        .v_flex()
        .bg(theme.background.opacity(0.35))
        .border_t_1()
        .border_color(theme.border)
        .child(kit::kv_grid(items, cx))
        .child(
            div().border_t_1().border_color(theme.border).child(kit::setting_row(
                "Connect automatically",
                None,
                Switch::new(SharedString::from(format!("vpn-auto-{}", vpn.uuid)))
                    .checked(vpn.autoconnect)
                    .on_click(cx.listener(move |this, on: &bool, window, cx| {
                        this.run(
                            format!("auto:{uuid_auto}"),
                            Action::SetProfileAutoconnect(uuid_auto.clone(), *on),
                            window,
                            cx,
                        )
                    })),
                0,
                cx,
            )),
        )
        .child(
            kit::actions()
                .px_3p5()
                .pb_3p5()
                .child(
                    kit::danger("vpn-forget", "Forget", armed).on_click(cx.listener(
                        move |this, _, window, cx| {
                            if this.confirm(&forget, cx) {
                                this.run(
                                    forget.clone(),
                                    Action::DeleteProfile(uuid.clone()),
                                    window,
                                    cx,
                                );
                            }
                        },
                    )),
                )
                .child(kit::plain("vpn-edit", "Edit profile").on_click(cx.listener(
                    move |this, _, window, cx| {
                        crate::ui::profiles::open(this, uuid_edit.clone(), window, cx)
                    },
                )))
                .child(div().flex_1()),
        )
}

pub fn render(app: &mut NmApp, _window: &mut Window, cx: &mut Context<NmApp>) -> AnyElement {
    let Some(state) = app.state.clone() else {
        return div().into_any_element();
    };
    let header = kit::header("VPN", cx)
        .child(
            ghost("vpn-import", IconName::FileUp, "Import OpenVPN").on_click(cx.listener(
                |this, _, window, cx| {
                    this.vpn.form = if this.vpn.form == Form::Import {
                        Form::None
                    } else {
                        Form::Import
                    };
                    if this.vpn.form == Form::Import {
                        let input = this.vpn.ovpn_path.clone();
                        input.update(cx, |s, cx| s.focus(window, cx));
                    }
                    cx.notify();
                },
            )),
        )
        .child(
            ghost("vpn-wg", IconName::Plus, "Add WireGuard").on_click(cx.listener(
                |this, _, window, cx| {
                    this.vpn.form = if this.vpn.form == Form::WireGuard {
                        Form::None
                    } else {
                        Form::WireGuard
                    };
                    if this.vpn.form == Form::WireGuard {
                        let input = this.vpn.wg_name.clone();
                        input.update(cx, |s, cx| s.focus(window, cx));
                    }
                    cx.notify();
                },
            )),
        );

    let mut body = div().v_flex().child(header);
    body = match app.vpn.form {
        Form::Import => body.child(import_form(app, cx)),
        Form::WireGuard => body.child(wireguard_form(app, cx)),
        Form::None => body,
    };

    if state.vpns.is_empty() {
        return body
            .child(kit::empty(
                IconName::ShieldOff,
                "No VPN connections",
                Some(
                    kit::plain("vpn-empty-wg", "Add WireGuard")
                        .icon(IconName::Plus)
                        .small()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.vpn.form = Form::WireGuard;
                            cx.notify();
                        })),
                ),
                cx,
            ))
            .into_any_element();
    }

    let mut vpns = state.vpns.clone();
    vpns.sort_by_key(|v| (!v.state.is_up(), v.name.to_lowercase()));
    body.child(kit::card(cx).children(vpns.iter().enumerate().map(|(i, v)| vpn_row(app, v, i, cx))))
        .into_any_element()
}
