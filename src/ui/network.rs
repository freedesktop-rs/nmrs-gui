use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::InputState;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Sizable, StyledExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::{NmApp, input, list, set, value};
use crate::model::Action;
use crate::model::*;
use crate::ui::kit;

pub struct NetworkUi {
    loaded: Option<GlobalDns>,
    servers: Entity<InputState>,
    search: Entity<InputState>,
    domains: Vec<(Entity<InputState>, Entity<InputState>)>,
}

impl NetworkUi {
    pub fn new(window: &mut Window, cx: &mut Context<NmApp>) -> Self {
        Self {
            loaded: None,
            servers: input("Per connection", false, window, cx),
            search: input("None", false, window, cx),
            domains: Vec::new(),
        }
    }
}

fn domain_row(
    window: &mut Window,
    cx: &mut Context<NmApp>,
) -> (Entity<InputState>, Entity<InputState>) {
    (
        input("corp.example", false, window, cx),
        input("10.0.0.53", false, window, cx),
    )
}

fn sync(app: &mut NmApp, dns: &GlobalDns, window: &mut Window, cx: &mut Context<NmApp>) {
    if app.network.loaded.as_ref() == Some(dns) {
        return;
    }
    let ui = &mut app.network;
    set(&ui.servers, &dns.servers.join(", "), window, cx);
    set(&ui.search, &dns.searches.join(", "), window, cx);
    ui.domains = dns
        .domains
        .iter()
        .map(|(name, servers)| {
            let (a, b) = domain_row(window, cx);
            set(&a, name, window, cx);
            set(&b, &servers.join(", "), window, cx);
            (a, b)
        })
        .collect();
    ui.loaded = Some(dns.clone());
}

fn read_dns(app: &NmApp, cx: &App) -> GlobalDns {
    let ui = &app.network;
    GlobalDns {
        servers: list(&ui.servers, cx),
        searches: list(&ui.search, cx),
        domains: ui
            .domains
            .iter()
            .map(|(name, servers)| (value(name, cx), list(servers, cx)))
            .filter(|(name, servers)| !name.is_empty() && !servers.is_empty())
            .collect(),
    }
}

fn radio_switch(
    id: &'static str,
    title: &'static str,
    detail: Option<String>,
    radio: Radio,
    index: usize,
    app: &NmApp,
    cx: &mut Context<NmApp>,
    action: fn(bool) -> Action,
) -> Div {
    let pending = app.is_pending(id);
    kit::setting_row(
        title,
        detail.or_else(|| (!radio.hardware).then(|| "Blocked by hardware switch".into())),
        Switch::new(id)
            .checked(radio.enabled)
            .disabled(!radio.hardware || pending)
            .on_click(cx.listener(move |this, on: &bool, window, cx| {
                this.run(id, action(*on), window, cx)
            })),
        index,
        cx,
    )
}

pub fn render(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) -> AnyElement {
    let Some(state) = app.state.clone() else {
        return div().into_any_element();
    };
    sync(app, &state.dns, window, cx);
    let theme = cx.theme().clone();

    let mut radios = kit::card(cx).child(kit::setting_row(
        "Airplane mode",
        None,
        Switch::new("airplane")
            .checked(state.airplane)
            .disabled(app.is_pending("airplane"))
            .on_click(cx.listener(|this, on: &bool, window, cx| {
                this.run("airplane", Action::SetAirplane(*on), window, cx)
            })),
        0,
        cx,
    ));
    let mut index = 1;
    for (id, title, radio, action) in [
        (
            "radio:wifi",
            "Wi-Fi",
            state.radios.wifi,
            Action::SetWifi as fn(bool) -> Action,
        ),
        (
            "radio:wwan",
            "Mobile broadband",
            state.radios.wwan,
            Action::SetWwan,
        ),
        (
            "radio:bt",
            "Bluetooth",
            state.radios.bluetooth,
            Action::SetBluetooth,
        ),
    ] {
        if radio.present {
            radios = radios.child(radio_switch(id, title, None, radio, index, app, cx, action));
            index += 1;
        }
    }

    let reach = state.connectivity.state;
    let color = match reach {
        Reachability::Full => theme.success,
        Reachability::Limited | Reachability::Portal => theme.warning,
        Reachability::None => theme.danger,
        Reachability::Unknown => theme.muted_foreground,
    };
    let checking = app.is_pending("check");
    let portal = state.connectivity.portal.clone();
    let connectivity = kit::card(cx).child(
        div()
            .h_flex()
            .items_center()
            .gap_3()
            .min_h_12()
            .px_3p5()
            .py_2()
            .child(div().size_2().rounded_full().bg(color))
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_sm().child(reach.label()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .truncate()
                            .child(
                                match (&state.connectivity.uri, state.connectivity.checking) {
                                    (Some(uri), true) => uri.clone(),
                                    _ => "Connectivity checking is disabled".into(),
                                },
                            ),
                    ),
            )
            .when_some(portal, |s, url| {
                s.child(
                    kit::primary("portal", "Sign in", false)
                        .icon(IconName::ExternalLink)
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                )
            })
            .child(
                kit::plain("check", "Check now")
                    .loading(checking)
                    .disabled(!state.connectivity.checking)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.run("check", Action::CheckConnectivity, window, cx)
                    })),
            ),
    );

    let dns_dirty = app
        .network
        .loaded
        .as_ref()
        .is_some_and(|l| *l != read_dns(app, cx));
    let saving = app.is_pending("dns");
    let dns = kit::card(cx).child(
        kit::form(cx)
            .child(kit::pair(
                kit::field("Servers", &app.network.servers, cx),
                kit::field("Search domains", &app.network.search, cx),
            ))
            .children(
                app.network
                    .domains
                    .iter()
                    .enumerate()
                    .map(|(i, (name, servers))| {
                        div()
                            .h_flex()
                            .items_end()
                            .gap_3()
                            .child(div().flex_1().child(kit::field("Domain", name, cx)))
                            .child(div().flex_1().child(kit::field("Servers", servers, cx)))
                            .child(
                                Button::new(SharedString::from(format!("dns-rm-{i}")))
                                    .ghost()
                                    .small()
                                    .icon(IconName::X)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if i < this.network.domains.len() {
                                            this.network.domains.remove(i);
                                        }
                                        cx.notify();
                                    })),
                            )
                    }),
            )
            .child(
                kit::actions()
                    .child(
                        kit::plain("dns-add", "Split domain")
                            .ghost()
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                let row = domain_row(window, cx);
                                this.network.domains.push(row);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        kit::plain("dns-clear", "Clear")
                            .ghost()
                            .disabled(state.dns == GlobalDns::default())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.network.loaded = None;
                                this.run(
                                    "dns",
                                    Action::SetGlobalDns(GlobalDns::default()),
                                    window,
                                    cx,
                                )
                            })),
                    )
                    .child(
                        kit::primary("dns-save", "Apply", saving)
                            .disabled(!dns_dirty)
                            .on_click(cx.listener(|this, _, window, cx| {
                                let dns = read_dns(this, cx);
                                this.network.loaded = None;
                                this.run("dns", Action::SetGlobalDns(dns), window, cx)
                            })),
                    ),
            ),
    );

    let mut devices = kit::card(cx);
    let mut idx = 0;
    for dev in &state.wifi_devices {
        devices = devices.child(
            div()
                .h_flex()
                .items_center()
                .gap_3()
                .min_h(app.row_height())
                .px_3p5()
                .map(kit::divided(idx, cx))
                .child(kit::badge_icon(IconName::Wifi, dev.state.is_up(), cx))
                .child(kit::title_block(
                    dev.interface.clone(),
                    Some(
                        [
                            Some(dev.state.label().to_string()),
                            dev.driver.clone(),
                            Some(dev.mac.clone()),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · "),
                    ),
                    cx,
                )),
        );
        idx += 1;
    }
    let show_virtual = app.config.devices.show_virtual;
    for dev in state
        .wired
        .iter()
        .filter(|d| show_virtual || !d.is_virtual())
    {
        devices = devices.child(
            div()
                .h_flex()
                .items_center()
                .gap_3()
                .min_h(app.row_height())
                .px_3p5()
                .map(kit::divided(idx, cx))
                .child(kit::badge_icon(
                    IconName::EthernetPort,
                    dev.state.is_up(),
                    cx,
                ))
                .child(kit::title_block(
                    dev.interface.clone(),
                    Some(
                        [
                            Some(dev.state.label().to_string()),
                            dev.driver.clone(),
                            Some(dev.mac.clone()),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · "),
                    ),
                    cx,
                )),
        );
        idx += 1;
    }

    div()
        .v_flex()
        .child(kit::header("Network", cx))
        .child(radios)
        .child(kit::section("Connectivity", cx))
        .child(connectivity)
        .child(kit::section("Global DNS", cx))
        .child(dns)
        .when(idx > 0, |s| {
            s.child(kit::section("Interfaces", cx)).child(devices)
        })
        .into_any_element()
}
