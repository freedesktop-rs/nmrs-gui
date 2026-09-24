use std::collections::BTreeMap;

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, StyledExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::{NmApp, input, list, set, value};
use crate::config::{LayoutMode, Page};
use crate::model::Action;
use crate::model::*;
use crate::ui::kit;

pub struct IpInputs {
    method: IpMethod,
    addresses: Entity<InputState>,
    gateway: Entity<InputState>,
    dns: Entity<InputState>,
    search: Entity<InputState>,
    ignore_auto_dns: bool,
    never_default: bool,
}

impl IpInputs {
    fn new(v6: bool, window: &mut Window, cx: &mut Context<NmApp>) -> Self {
        let (addr, gw, dns) = if v6 {
            ("2001:db8::10/64", "2001:db8::1", "2606:4700:4700::1111")
        } else {
            ("192.168.1.10/24", "192.168.1.1", "1.1.1.1, 9.9.9.9")
        };
        Self {
            method: IpMethod::Auto,
            addresses: input(addr, false, window, cx),
            gateway: input(gw, false, window, cx),
            dns: input(dns, false, window, cx),
            search: input("example.lan", false, window, cx),
            ignore_auto_dns: false,
            never_default: false,
        }
    }

    fn load(&mut self, ip: &IpConfig, window: &mut Window, cx: &mut Context<NmApp>) {
        self.method = ip.method;
        self.ignore_auto_dns = ip.ignore_auto_dns;
        self.never_default = ip.never_default;
        set(&self.addresses, &ip.addresses.join(", "), window, cx);
        set(
            &self.gateway,
            ip.gateway.as_deref().unwrap_or_default(),
            window,
            cx,
        );
        set(&self.dns, &ip.dns.join(", "), window, cx);
        set(&self.search, &ip.search.join(", "), window, cx);
    }

    fn read(&self, cx: &App) -> IpConfig {
        IpConfig {
            method: self.method,
            addresses: list(&self.addresses, cx),
            gateway: Some(value(&self.gateway, cx)).filter(|s| !s.is_empty()),
            dns: list(&self.dns, cx),
            search: list(&self.search, cx),
            ignore_auto_dns: self.ignore_auto_dns,
            never_default: self.never_default,
        }
    }
}

pub struct ProfilesUi {
    search: Entity<InputState>,
    pub selected: Option<String>,
    detail: Option<ProfileDetail>,
    original: Option<ProfileEdit>,
    loading: bool,
    name: Entity<InputState>,
    interface: Entity<InputState>,
    priority: Entity<InputState>,
    autoconnect: bool,
    v4: IpInputs,
    v6: IpInputs,
    show_raw: bool,
}

impl ProfilesUi {
    pub fn new(window: &mut Window, cx: &mut Context<NmApp>) -> Self {
        Self {
            search: input("Search", false, window, cx),
            selected: None,
            detail: None,
            original: None,
            loading: false,
            name: input("Name", false, window, cx),
            interface: input("Any", false, window, cx),
            priority: input("0", false, window, cx),
            autoconnect: true,
            v4: IpInputs::new(false, window, cx),
            v6: IpInputs::new(true, window, cx),
            show_raw: false,
        }
    }
}

pub fn subscribe(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let watched = [
        app.profiles.search.clone(),
        app.profiles.name.clone(),
        app.profiles.interface.clone(),
        app.profiles.priority.clone(),
        app.profiles.v4.addresses.clone(),
        app.profiles.v4.gateway.clone(),
        app.profiles.v4.dns.clone(),
        app.profiles.v4.search.clone(),
        app.profiles.v6.addresses.clone(),
        app.profiles.v6.gateway.clone(),
        app.profiles.v6.dns.clone(),
        app.profiles.v6.search.clone(),
    ];
    for input in watched {
        let sub = cx.subscribe_in(&input, window, |_, _, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                cx.notify();
            }
        });
        app.subscriptions.push(sub);
    }
}

pub fn open(app: &mut NmApp, uuid: String, window: &mut Window, cx: &mut Context<NmApp>) {
    if app.config.layout.pages.contains(&Page::Profiles) {
        app.page = Page::Profiles;
    }
    select(app, uuid, window, cx);
}

pub fn ensure_selection(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    if app.profiles.selected.is_some() || app.config.layout.mode == LayoutMode::Compact {
        return;
    }
    let first = app.state.as_ref().and_then(|s| {
        sorted(&s.profiles, "")
            .into_iter()
            .next()
            .map(|p| p.uuid.clone())
    });
    if let Some(uuid) = first {
        select(app, uuid, window, cx);
    }
}

fn select(app: &mut NmApp, uuid: String, window: &mut Window, cx: &mut Context<NmApp>) {
    app.profiles.selected = Some(uuid.clone());
    app.profiles.loading = true;
    app.confirming = None;
    let fut = app.backend.profile(uuid.clone());
    cx.notify();
    cx.spawn_in(window, async move |this, cx| {
        let result = fut.await;
        this.update_in(cx, |this, window, cx| {
            if this.profiles.selected.as_deref() != Some(uuid.as_str()) {
                return;
            }
            this.profiles.loading = false;
            match result {
                Ok(detail) => load(this, detail, window, cx),
                Err(err) => {
                    this.profiles.detail = None;
                    use gpui_kit::component::WindowExt;
                    window.push_notification(
                        gpui_kit::component::notification::Notification::error(format!("{err:#}"))
                            .title("Profile"),
                        cx,
                    );
                }
            }
            cx.notify();
        })
        .ok();
    })
    .detach();
}

fn load(app: &mut NmApp, detail: ProfileDetail, window: &mut Window, cx: &mut Context<NmApp>) {
    let ui = &mut app.profiles;
    let p = &detail.profile;
    set(&ui.name, &p.name, window, cx);
    set(
        &ui.interface,
        p.interface.as_deref().unwrap_or_default(),
        window,
        cx,
    );
    set(&ui.priority, &p.priority.to_string(), window, cx);
    ui.autoconnect = p.autoconnect;
    ui.v4.load(&detail.ipv4, window, cx);
    ui.v6.load(&detail.ipv6, window, cx);
    ui.original = Some(ProfileEdit {
        name: p.name.clone(),
        interface: p.interface.clone().filter(|s| !s.is_empty()),
        autoconnect: p.autoconnect,
        priority: p.priority,
        ipv4: detail.ipv4.clone(),
        ipv6: detail.ipv6.clone(),
    });
    ui.detail = Some(detail);
}

fn edit(app: &NmApp, cx: &App) -> ProfileEdit {
    let ui = &app.profiles;
    ProfileEdit {
        name: value(&ui.name, cx),
        interface: Some(value(&ui.interface, cx)).filter(|s| !s.is_empty()),
        autoconnect: ui.autoconnect,
        priority: value(&ui.priority, cx).parse().unwrap_or(0),
        ipv4: ui.v4.read(cx),
        ipv6: ui.v6.read(cx),
    }
}

fn dirty(app: &NmApp, cx: &App) -> bool {
    app.profiles
        .original
        .as_ref()
        .is_some_and(|o| *o != edit(app, cx))
}

fn save(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let Some(uuid) = app.profiles.selected.clone() else {
        return;
    };
    let patch = edit(app, cx);
    if patch.name.is_empty() {
        return;
    }
    app.run_then(
        format!("save:{uuid}"),
        Action::UpdateProfile(uuid.clone(), Box::new(patch)),
        window,
        cx,
        move |this, window, cx| select(this, uuid, window, cx),
    );
}

fn revert(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    if let Some(detail) = app.profiles.detail.clone() {
        load(app, detail, window, cx);
        cx.notify();
    }
}

fn sorted<'a>(profiles: &'a [Profile], query: &str) -> Vec<&'a Profile> {
    let query = query.to_lowercase();
    let mut out: Vec<&Profile> = profiles
        .iter()
        .filter(|p| p.kind != ProfileKind::Loopback)
        .filter(|p| {
            query.is_empty()
                || p.name.to_lowercase().contains(&query)
                || p.kind.label().to_lowercase().contains(&query)
        })
        .collect();
    out.sort_by_key(|p| {
        (
            !p.active,
            std::cmp::Reverse(p.last_used),
            p.name.to_lowercase(),
        )
    });
    out
}

fn kind_icon(kind: &ProfileKind) -> IconName {
    match kind {
        ProfileKind::Wifi => IconName::Wifi,
        ProfileKind::Ethernet => IconName::EthernetPort,
        ProfileKind::Vpn | ProfileKind::WireGuard => IconName::Shield,
        ProfileKind::Bluetooth => IconName::Bluetooth,
        ProfileKind::Mobile => IconName::RadioTower,
        ProfileKind::Bridge | ProfileKind::Bond | ProfileKind::Vlan => IconName::Network,
        _ => IconName::Bookmark,
    }
}

fn relative(ts: u64) -> Option<String> {
    if ts == 0 {
        return None;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(ts);
    let d = now.saturating_sub(ts);
    Some(match d {
        0..=119 => "just now".into(),
        120..=7199 => format!("{} min ago", d / 60),
        7200..=172_799 => format!("{} h ago", d / 3600),
        _ => format!("{} days ago", d / 86_400),
    })
}

fn render_list(app: &NmApp, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let Some(state) = app.state.as_ref() else {
        return div();
    };
    let query = value(&app.profiles.search, cx);
    let profiles = sorted(&state.profiles, &query);
    let mut groups: BTreeMap<(u8, String), Vec<&Profile>> = BTreeMap::new();
    for p in profiles {
        let rank = match p.kind {
            ProfileKind::Wifi => 0,
            ProfileKind::Ethernet => 1,
            ProfileKind::Vpn | ProfileKind::WireGuard => 2,
            _ => 3,
        };
        let label = match p.kind {
            ProfileKind::WireGuard => "VPN".to_string(),
            ref k => k.label().to_string(),
        };
        groups.entry((rank, label)).or_default().push(p);
    }
    let selected = app.profiles.selected.clone();
    let radius = theme.radius;

    div()
        .v_flex()
        .gap_0p5()
        .children(groups.into_iter().map(|((_, label), items)| {
            div()
                .v_flex()
                .gap_0p5()
                .child(
                    div()
                        .px_2()
                        .pt_3()
                        .pb_1()
                        .text_xs()
                        .font_medium()
                        .text_color(theme.muted_foreground)
                        .child(label),
                )
                .children(items.into_iter().map(|p| {
                    let uuid = p.uuid.clone();
                    let active = selected.as_deref() == Some(p.uuid.as_str());
                    div()
                        .id(SharedString::from(format!("prof-{}", p.uuid)))
                        .h_flex()
                        .items_center()
                        .gap_2p5()
                        .h_9()
                        .px_2()
                        .rounded(radius)
                        .cursor_pointer()
                        .text_sm()
                        .when(active, |s| s.bg(theme.accent))
                        .when(!active, |s| s.hover(|s| s.bg(theme.accent.opacity(0.5))))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            select(this, uuid.clone(), window, cx)
                        }))
                        .child(
                            Icon::new(kind_icon(&p.kind))
                                .size_4()
                                .text_color(if p.active {
                                    theme.primary
                                } else {
                                    theme.muted_foreground
                                }),
                        )
                        .child(div().flex_1().min_w_0().truncate().child(p.name.clone()))
                        .when(p.active, |s| {
                            s.child(div().size_1p5().rounded_full().bg(theme.primary))
                        })
                }))
        }))
}

fn ip_card(
    title: &'static str,
    id: &'static str,
    v6: bool,
    app: &NmApp,
    cx: &mut Context<NmApp>,
) -> Div {
    let ip = if v6 {
        &app.profiles.v6
    } else {
        &app.profiles.v4
    };
    let methods: Vec<(IpMethod, &'static str)> = IpMethod::ALL
        .iter()
        .filter(|m| !(v6 && **m == IpMethod::Shared))
        .map(|m| (*m, m.label()))
        .collect();
    let manual = ip.method == IpMethod::Manual;
    let configurable = !matches!(ip.method, IpMethod::Disabled);
    let theme = cx.theme().clone();
    kit::card(cx)
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_3()
                .px_3p5()
                .py_2p5()
                .child(div().flex_1().text_sm().font_medium().child(title))
                .child(kit::segmented(
                    id,
                    &methods,
                    ip.method,
                    cx,
                    move |this, m, _, cx| {
                        if v6 {
                            this.profiles.v6.method = m;
                        } else {
                            this.profiles.v4.method = m;
                        }
                        cx.notify();
                    },
                )),
        )
        .when(configurable, |s| {
            s.child(
                kit::form(cx)
                    .border_t_1()
                    .border_color(theme.border)
                    .when(manual, |s| {
                        s.child(kit::pair(
                            kit::field("Addresses", &ip.addresses, cx),
                            kit::field("Gateway", &ip.gateway, cx),
                        ))
                    })
                    .child(kit::pair(
                        kit::field("DNS servers", &ip.dns, cx),
                        kit::field("Search domains", &ip.search, cx),
                    )),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(kit::setting_row(
                        "Ignore automatic DNS",
                        None,
                        Switch::new(SharedString::from(format!("{id}-iad")))
                            .checked(ip.ignore_auto_dns)
                            .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                if v6 {
                                    this.profiles.v6.ignore_auto_dns = *on;
                                } else {
                                    this.profiles.v4.ignore_auto_dns = *on;
                                }
                                cx.notify();
                            })),
                        0,
                        cx,
                    ))
                    .child(kit::setting_row(
                        "Never use as default route",
                        None,
                        Switch::new(SharedString::from(format!("{id}-nd")))
                            .checked(ip.never_default)
                            .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                if v6 {
                                    this.profiles.v6.never_default = *on;
                                } else {
                                    this.profiles.v4.never_default = *on;
                                }
                                cx.notify();
                            })),
                        1,
                        cx,
                    )),
            )
        })
}

fn raw_settings(detail: &ProfileDetail, cx: &App) -> Div {
    let theme = cx.theme();
    let mono = theme.mono_font_family.clone();
    kit::card(cx).children(
        detail
            .sections
            .iter()
            .enumerate()
            .map(|(i, (name, entries))| {
                div()
                    .v_flex()
                    .map(kit::divided(i, cx))
                    .px_3p5()
                    .py_2p5()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(theme.primary)
                            .pb_1()
                            .child(name.clone()),
                    )
                    .children(entries.iter().map(|(k, v)| {
                        div()
                            .h_flex()
                            .gap_4()
                            .text_xs()
                            .font_family(mono.clone())
                            .child(
                                div()
                                    .w(px(200.))
                                    .flex_shrink_0()
                                    .text_color(theme.muted_foreground)
                                    .truncate()
                                    .child(k.clone()),
                            )
                            .child(div().flex_1().min_w_0().truncate().child(v.clone()))
                    }))
            }),
    )
}

fn render_editor(app: &NmApp, compact: bool, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let Some(detail) = app.profiles.detail.clone() else {
        let msg = if app.profiles.loading || app.profiles.selected.is_some() {
            ""
        } else {
            "Select a profile"
        };
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(msg);
    };
    let p = detail.profile.clone();
    let uuid = p.uuid.clone();
    let dirty = dirty(app, cx);
    let saving = app.is_pending(&format!("save:{uuid}"));
    let toggling = app.is_pending(&format!("prof:{uuid}"));
    let forget = format!("forget:{uuid}");
    let armed = app.is_confirming(&forget);
    let active = app
        .state
        .as_ref()
        .and_then(|s| s.profiles.iter().find(|x| x.uuid == uuid))
        .map(|x| x.active)
        .unwrap_or(p.active);

    let mut meta = vec![p.kind.label().to_string()];
    if let Some(when) = relative(p.last_used) {
        meta.push(format!("Used {when}"));
    }

    let (uuid_a, uuid_b) = (uuid.clone(), uuid.clone());
    let header = div()
        .h_flex()
        .items_center()
        .gap_3()
        .mb_5()
        .when(compact, |s| {
            s.child(
                Button::new("prof-back")
                    .ghost()
                    .small()
                    .icon(IconName::ChevronLeft)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.profiles.selected = None;
                        this.profiles.detail = None;
                        cx.notify();
                    })),
            )
        })
        .child(kit::badge_icon(kind_icon(&p.kind), active, cx))
        .child(
            div()
                .v_flex()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_lg()
                        .font_semibold()
                        .truncate()
                        .child(p.name.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(meta.join(" · ")),
                ),
        )
        .child(
            kit::plain(
                "prof-toggle",
                if active { "Deactivate" } else { "Activate" },
            )
            .loading(toggling)
            .on_click(cx.listener(move |this, _, window, cx| {
                let action = if active {
                    Action::Deactivate(uuid_a.clone())
                } else {
                    Action::Activate(uuid_a.clone())
                };
                this.run(format!("prof:{uuid_a}"), action, window, cx);
            })),
        );

    let general = kit::card(cx)
        .child(
            kit::form(cx)
                .child(kit::pair(
                    kit::field("Name", &app.profiles.name, cx),
                    kit::field("Interface", &app.profiles.interface, cx),
                ))
                .child(kit::pair(
                    kit::field("Priority", &app.profiles.priority, cx),
                    div(),
                )),
        )
        .child(
            div()
                .border_t_1()
                .border_color(theme.border)
                .child(kit::setting_row(
                    "Connect automatically",
                    None,
                    Switch::new("prof-auto")
                        .checked(app.profiles.autoconnect)
                        .on_click(cx.listener(|this, on: &bool, _, cx| {
                            this.profiles.autoconnect = *on;
                            cx.notify();
                        })),
                    0,
                    cx,
                )),
        );

    let mut facts = vec![("UUID", p.uuid.clone())];
    if let Some(m) = &detail.mac_randomization {
        facts.push(("MAC randomization", m.clone()));
    }
    if let Some(m) = detail.metered {
        facts.push(("Metered", if m { "Yes" } else { "No" }.into()));
    }

    let show_raw = app.profiles.show_raw;
    div()
        .v_flex()
        .w_full()
        .child(header)
        .child(general)
        .child(kit::section("Addressing", cx))
        .child(
            div()
                .v_flex()
                .gap_3()
                .child(ip_card("IPv4", "ip4", false, app, cx))
                .child(ip_card("IPv6", "ip6", true, app, cx)),
        )
        .child(kit::section("Details", cx))
        .child(kit::card(cx).child(kit::kv_grid(facts, cx)))
        .child(
            kit::section("All settings", cx)
                .child(div().flex_1())
                .child(
                    Button::new("raw-toggle")
                        .ghost()
                        .xsmall()
                        .icon(if show_raw {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.profiles.show_raw = !this.profiles.show_raw;
                            cx.notify();
                        })),
                ),
        )
        .when(show_raw, |s| s.child(raw_settings(&detail, cx)))
        .child(
            kit::actions()
                .mt_6()
                .child(
                    kit::danger("prof-forget", "Forget", armed).on_click(cx.listener(
                        move |this, _, window, cx| {
                            if this.confirm(&forget, cx) {
                                let uuid = uuid_b.clone();
                                this.run_then(
                                    forget.clone(),
                                    Action::DeleteProfile(uuid),
                                    window,
                                    cx,
                                    |this, _, cx| {
                                        this.profiles.selected = None;
                                        this.profiles.detail = None;
                                        cx.notify();
                                    },
                                );
                            }
                        },
                    )),
                )
                .child(div().flex_1())
                .child(
                    kit::plain("prof-revert", "Revert")
                        .ghost()
                        .disabled(!dirty)
                        .on_click(cx.listener(|this, _, window, cx| revert(this, window, cx))),
                )
                .child(
                    kit::primary("prof-save", "Save", saving)
                        .disabled(!dirty)
                        .on_click(cx.listener(|this, _, window, cx| save(this, window, cx))),
                ),
        )
}

use gpui_kit::component::button::{Button, ButtonVariants};

pub fn render(app: &mut NmApp, _window: &mut Window, cx: &mut Context<NmApp>) -> AnyElement {
    let compact = app.config.layout.mode == LayoutMode::Compact;
    let theme = cx.theme();
    if compact {
        if app.profiles.selected.is_some() {
            return render_editor(app, true, cx).into_any_element();
        }
        return div()
            .v_flex()
            .child(kit::header("Profiles", cx))
            .child(
                Input::new(&app.profiles.search)
                    .small()
                    .prefix(Icon::new(IconName::Search).size_3p5()),
            )
            .child(render_list(app, cx))
            .into_any_element();
    }

    div()
        .h_flex()
        .size_full()
        .child(
            div()
                .v_flex()
                .w(px(272.))
                .h_full()
                .flex_shrink_0()
                .border_r_1()
                .border_color(theme.border)
                .child(
                    div()
                        .px_3()
                        .pt_5()
                        .pb_2()
                        .child(
                            div()
                                .text_xl()
                                .font_semibold()
                                .px_1()
                                .pb_3()
                                .child("Profiles"),
                        )
                        .child(
                            Input::new(&app.profiles.search)
                                .small()
                                .prefix(Icon::new(IconName::Search).size_3p5()),
                        ),
                )
                .child(
                    div()
                        .id("prof-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .child(div().px_3().pb_4().child(render_list(app, cx))),
                ),
        )
        .child(
            div()
                .id("prof-editor")
                .flex_1()
                .h_full()
                .min_w_0()
                .overflow_y_scrollbar()
                .child(
                    div()
                        .w_full()
                        .max_w(px(680.))
                        .mx_auto()
                        .px(px(28.))
                        .pt(px(28.))
                        .pb(px(40.))
                        .child(render_editor(app, false, cx)),
                ),
        )
        .into_any_element()
}
