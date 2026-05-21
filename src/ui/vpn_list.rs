use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, GestureClick, Image, Label, ListBox, ListBoxRow, Orientation};
use nmrs::VpnConnection;
use std::rc::Rc;

use crate::ui::networks::NetworksContext;
use crate::ui::vpn_details_page::VpnDetailsPage;

pub fn vpn_section(
    ctx: Rc<NetworksContext>,
    vpns: &[VpnConnection],
    details_page: Rc<VpnDetailsPage>,
    list_container: &GtkBox,
) {
    if vpns.is_empty() {
        return;
    }

    let separator = gtk::Separator::new(Orientation::Horizontal);
    separator.add_css_class("device-separator");
    separator.set_margin_top(12);
    separator.set_margin_bottom(12);
    list_container.append(&separator);

    let header = Label::new(Some("VPN"));
    header.add_css_class("section-header");
    header.add_css_class("vpn-section-header");
    header.set_halign(Align::Start);
    header.set_margin_top(8);
    header.set_margin_bottom(4);
    header.set_margin_start(12);
    list_container.append(&header);

    let list = vpn_list_view(ctx, vpns, details_page);
    list.add_css_class("vpn-list");
    list_container.append(&list);
}

pub fn vpn_add_button(ctx: &NetworksContext, list_container: &GtkBox) {
    let row = ListBoxRow::new();
    row.add_css_class("network-selection");
    row.add_css_class("vpn-add-row");

    let hbox = GtkBox::new(Orientation::Horizontal, 6);

    let icon = Image::from_icon_name("list-add-symbolic");
    icon.add_css_class("vpn-add-icon");
    hbox.append(&icon);

    let label = Label::new(Some("Add VPN"));
    label.add_css_class("vpn-add-label");
    hbox.append(&label);

    row.set_child(Some(&hbox));

    let click = GestureClick::new();
    let stack = ctx.stack.clone();
    click.connect_pressed(move |_, _, _, _| {
        stack.set_visible_child_name("vpn-add");
    });
    row.add_controller(click);

    let add_list = ListBox::new();
    add_list.add_css_class("vpn-list");
    add_list.append(&row);
    list_container.append(&add_list);
}

fn vpn_list_view(
    ctx: Rc<NetworksContext>,
    vpns: &[VpnConnection],
    details_page: Rc<VpnDetailsPage>,
) -> ListBox {
    let list = ListBox::new();

    for vpn in vpns {
        let row = ListBoxRow::new();
        let hbox = GtkBox::new(Orientation::Horizontal, 6);

        row.add_css_class("network-selection");

        if vpn.active {
            row.add_css_class("connected");
        }

        let name_label = Label::new(Some(&vpn.name));
        hbox.append(&name_label);

        let type_label = Label::new(Some(vpn_type_short(&vpn.vpn_type)));
        type_label.add_css_class("vpn-type-label");
        hbox.append(&type_label);

        if vpn.active {
            let connected_label = Label::new(Some("Connected"));
            connected_label.add_css_class("connected-label");
            hbox.append(&connected_label);
        }

        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        hbox.append(&spacer);

        let icon = Image::from_icon_name("network-vpn-symbolic");
        icon.add_css_class("vpn-icon");
        hbox.append(&icon);

        let arrow = Image::from_icon_name("go-next-symbolic");
        arrow.set_halign(Align::End);
        arrow.add_css_class("network-arrow");
        arrow.set_cursor_from_name(Some("pointer"));
        hbox.append(&arrow);

        row.set_child(Some(&hbox));

        // Arrow click -> details page
        {
            let click = GestureClick::new();
            let vpn_clone = vpn.clone();
            let ctx_c = ctx.clone();
            let page = details_page.clone();

            click.connect_pressed(move |_, _, _, _| {
                let vpn = vpn_clone.clone();
                let ctx = ctx_c.clone();
                let page = page.clone();

                glib::MainContext::default().spawn_local(async move {
                    page.update(&vpn);

                    if vpn.active {
                        if let Ok(info) = ctx.nm.get_vpn_info(&vpn.name).await {
                            page.enrich_with_info(&info);
                        }
                    }

                    ctx.stack.set_visible_child_name("vpn-details");
                });
            });

            arrow.add_controller(click);
        }

        // Double-click row -> connect/disconnect
        {
            let click = GestureClick::new();
            let vpn_clone = vpn.clone();
            let ctx_c = ctx.clone();

            click.connect_pressed(move |_, n, _, _| {
                if n != 2 {
                    return;
                }

                let vpn = vpn_clone.clone();
                let ctx = ctx_c.clone();
                let status = ctx.status.clone();
                let window = ctx.parent_window.clone();
                let on_success = ctx.on_success.clone();

                glib::MainContext::default().spawn_local(async move {
                    window.set_sensitive(false);

                    let result = if vpn.active {
                        status.set_text(&format!("Disconnecting {}...", vpn.name));
                        ctx.nm.disconnect_vpn_by_uuid(&vpn.uuid).await
                    } else {
                        status.set_text(&format!("Connecting to {}...", vpn.name));
                        ctx.nm.connect_vpn_by_uuid(&vpn.uuid).await
                    };

                    match result {
                        Ok(_) => {
                            status.set_text("");
                            on_success();
                        }
                        Err(e) => {
                            status.set_text(&format!("VPN error: {e}"));
                        }
                    }

                    window.set_sensitive(true);
                });
            });

            row.add_controller(click);
        }

        list.append(&row);
    }

    list
}

fn vpn_type_short(vpn_type: &nmrs::VpnType) -> &'static str {
    match vpn_type {
        nmrs::VpnType::WireGuard { .. } => "WireGuard",
        nmrs::VpnType::OpenVpn { .. } => "OpenVPN",
        nmrs::VpnType::OpenConnect { .. } => "OpenConnect",
        nmrs::VpnType::StrongSwan { .. } => "strongSwan",
        nmrs::VpnType::Pptp { .. } => "PPTP",
        nmrs::VpnType::L2tp { .. } => "L2TP",
        _ => "VPN",
    }
}
