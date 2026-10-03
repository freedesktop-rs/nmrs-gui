use gpui_kit::assets::IconName;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, StyledExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::NmApp;
use crate::model::Action;
use crate::model::*;
use crate::ui::kit;

fn speed(mbps: u32) -> String {
    if mbps >= 1000 && mbps % 1000 == 0 {
        format!("{} Gb/s", mbps / 1000)
    } else if mbps >= 1000 {
        format!("{:.1} Gb/s", mbps as f32 / 1000.0)
    } else {
        format!("{mbps} Mb/s")
    }
}

fn device_card(app: &NmApp, dev: &WiredDevice, cx: &mut Context<NmApp>) -> Div {
    let theme = cx.theme();
    let key = format!("wired:{}", dev.interface);
    let pending = app.is_pending(&key);
    let up = dev.state.is_up();
    let subtitle = match (&dev.profile, dev.state) {
        (Some(profile), LinkState::Connected) => format!("Connected · {profile}"),
        (_, state) => state.label().to_string(),
    };

    let mut items = Vec::new();
    if let Some(ip) = &dev.ip4 {
        items.push(("IPv4", ip.clone()));
    }
    if let Some(ip) = &dev.ip6 {
        items.push(("IPv6", ip.clone()));
    }
    if let Some(s) = dev.speed_mbps.filter(|s| *s > 0) {
        items.push(("Link speed", speed(s)));
    }
    items.push(("Hardware address", dev.mac.clone()));
    if let Some(driver) = &dev.driver {
        items.push(("Driver", driver.clone()));
    }

    let uuid = dev.profile_uuid.clone();
    let iface = dev.interface.clone();
    let iface_managed = dev.interface.clone();
    let iface_auto = dev.interface.clone();
    let profile_uuid = dev.profile_uuid.clone();
    let can_connect = matches!(dev.state, LinkState::Disconnected | LinkState::Failed);

    kit::card(cx)
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_3()
                .min_h(app.row_height())
                .px_3p5()
                .py_2()
                .child(kit::badge_icon(IconName::EthernetPort, up, cx))
                .child(kit::title_block(dev.interface.clone(), Some(subtitle), cx))
                .when(up, |s| {
                    s.child(
                        kit::plain("wired-down", "Disconnect")
                            .loading(pending)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                let action = match uuid.clone() {
                                    Some(uuid) => Action::Deactivate(uuid),
                                    None => Action::DisconnectDevice(iface.clone()),
                                };
                                this.run(format!("wired:{iface}"), action, window, cx);
                            })),
                    )
                })
                .when(!up && can_connect, |s| {
                    let iface = dev.interface.clone();
                    s.child(
                        kit::primary("wired-up", "Connect", pending).on_click(cx.listener(
                            move |this, _, window, cx| {
                                let profile = this.state.as_ref().and_then(|st| {
                                    st.profiles
                                        .iter()
                                        .filter(|p| p.kind == ProfileKind::Ethernet)
                                        .filter(|p| {
                                            p.interface.as_ref().is_none_or(|i| *i == iface)
                                        })
                                        .max_by_key(|p| p.last_used)
                                        .map(|p| p.uuid.clone())
                                });
                                if let Some(uuid) = profile {
                                    this.run(
                                        format!("wired:{iface}"),
                                        Action::Activate(uuid),
                                        window,
                                        cx,
                                    );
                                }
                            },
                        )),
                    )
                }),
        )
        .child(
            div()
                .border_t_1()
                .border_color(theme.border)
                .child(kit::kv_grid(items, cx)),
        )
        .child(
            div()
                .border_t_1()
                .border_color(theme.border)
                .child(kit::setting_row(
                    "Managed",
                    None,
                    Switch::new(SharedString::from(format!("managed-{}", dev.interface)))
                        .checked(dev.managed)
                        .on_click(cx.listener(move |this, on: &bool, window, cx| {
                            this.run(
                                format!("managed:{iface_managed}"),
                                Action::SetManaged(iface_managed.clone(), *on),
                                window,
                                cx,
                            )
                        })),
                    0,
                    cx,
                ))
                .child(kit::setting_row(
                    "Connect automatically",
                    None,
                    Switch::new(SharedString::from(format!("auto-{}", dev.interface)))
                        .checked(dev.autoconnect)
                        .on_click(cx.listener(move |this, on: &bool, window, cx| {
                            this.run(
                                format!("auto:{iface_auto}"),
                                Action::SetDeviceAutoconnect(iface_auto.clone(), *on),
                                window,
                                cx,
                            )
                        })),
                    1,
                    cx,
                )),
        )
        .when_some(profile_uuid, |s, uuid| {
            s.child(
                kit::actions()
                    .px_3p5()
                    .pb_3()
                    .border_t_1()
                    .border_color(theme.border)
                    .pt_3()
                    .child(
                        kit::plain("wired-edit", "Edit profile").on_click(cx.listener(
                            move |this, _, window, cx| {
                                crate::ui::profiles::open(this, uuid.clone(), window, cx)
                            },
                        )),
                    ),
            )
        })
}

pub fn render(app: &mut NmApp, _window: &mut Window, cx: &mut Context<NmApp>) -> AnyElement {
    let Some(state) = app.state.clone() else {
        return div().into_any_element();
    };
    let body = div().v_flex().child(kit::header("Ethernet", cx));
    let show_virtual = app.config.devices.show_virtual;
    let devices: Vec<WiredDevice> = state
        .wired
        .iter()
        .filter(|d| show_virtual || !d.is_virtual())
        .cloned()
        .collect();
    if devices.is_empty() {
        return body
            .child(kit::empty(
                IconName::Cable,
                "No Ethernet adapters",
                None,
                cx,
            ))
            .into_any_element();
    }
    let gap = app.gap();
    body.child(
        div()
            .v_flex()
            .gap(gap)
            .children(devices.iter().map(|d| device_card(app, d, cx))),
    )
    .into_any_element()
}
