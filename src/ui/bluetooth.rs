use gpui_kit::assets::IconName;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Disableable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::NmApp;
use crate::model::Action;
use crate::model::*;
use crate::ui::kit;

pub fn render(app: &mut NmApp, _window: &mut Window, cx: &mut Context<NmApp>) -> AnyElement {
    let Some(state) = app.state.clone() else {
        return div().into_any_element();
    };
    let radio = state.radios.bluetooth;
    let toggling = app.is_pending("radio:bt");
    let mut header = kit::header("Bluetooth", cx);
    if radio.present {
        header = header.child(
            Switch::new("bt-radio")
                .checked(radio.enabled)
                .disabled(!radio.hardware || toggling)
                .on_click(cx.listener(|this, on: &bool, window, cx| {
                    this.run("radio:bt", Action::SetBluetooth(*on), window, cx)
                })),
        );
    }
    let body = div().v_flex().child(header);

    if !radio.present {
        return body
            .child(kit::empty(
                IconName::BluetoothOff,
                "No Bluetooth adapter",
                None,
                cx,
            ))
            .into_any_element();
    }
    if !radio.enabled {
        return body
            .child(kit::empty(
                IconName::BluetoothOff,
                "Bluetooth is off",
                Some(
                    kit::primary("bt-on", "Turn on", toggling).on_click(cx.listener(
                        |this, _, window, cx| {
                            this.run("radio:bt", Action::SetBluetooth(true), window, cx)
                        },
                    )),
                ),
                cx,
            ))
            .into_any_element();
    }
    let devices: Vec<BluetoothDevice> = state
        .bluetooth
        .iter()
        .filter(|d| d.nap || d.dun)
        .cloned()
        .collect();
    if devices.is_empty() {
        return body
            .child(kit::empty(
                IconName::BluetoothSearching,
                "No paired devices offer network access",
                None,
                cx,
            ))
            .into_any_element();
    }

    let height = app.row_height();
    body.child(kit::section("Tethering", cx).mt_0())
        .child(
            kit::card(cx).children(devices.into_iter().enumerate().map(|(i, dev)| {
                let up = dev.state.is_up();
                let key = format!("bt:{}", dev.address);
                let pending = app.is_pending(&key);
                let address = dev.address.clone();
                let name = dev.name.clone();
                let dun = !dev.nap && dev.dun;
                let mut caps = Vec::new();
                if dev.nap {
                    caps.push("PAN");
                }
                if dev.dun {
                    caps.push("DUN");
                }
                div().map(kit::divided(i, cx)).child(
                    kit::row(
                        SharedString::from(format!("bt-{}", dev.address)),
                        height,
                        cx,
                    )
                    .child(kit::badge_icon(
                        if up {
                            IconName::BluetoothConnected
                        } else {
                            IconName::Bluetooth
                        },
                        up,
                        cx,
                    ))
                    .child(kit::title_block(
                        dev.name.clone(),
                        Some(if up {
                            format!("Connected · {}", dev.address)
                        } else {
                            dev.address.clone()
                        }),
                        cx,
                    ))
                    .children(caps.into_iter().map(|c| kit::tag(c, cx)))
                    .child(if up {
                        kit::plain(SharedString::from(format!("btd-{i}")), "Connected")
                            .disabled(true)
                    } else {
                        kit::primary(SharedString::from(format!("btc-{i}")), "Connect", pending)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.run(
                                    key.clone(),
                                    Action::ConnectBluetooth {
                                        address: address.clone(),
                                        name: name.clone(),
                                        dun,
                                    },
                                    window,
                                    cx,
                                )
                            }))
                    }),
                )
            })),
        )
        .into_any_element()
}
