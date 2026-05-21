use glib::clone;
use gtk::prelude::*;
use gtk::{Align, Box, Button, Image, Label, Orientation};
use nmrs::{NetworkManager, VpnConnection, VpnConnectionInfo, VpnDetails};
use std::cell::RefCell;
use std::rc::Rc;

type OnSuccessCallback = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

pub struct VpnDetailsPage {
    root: gtk::Box,

    title: gtk::Label,
    status_val: gtk::Label,
    vpn_type_val: gtk::Label,
    interface_val: gtk::Label,

    network_section: gtk::Box,
    ip4_val: gtk::Label,
    ip6_val: gtk::Label,
    gateway_val: gtk::Label,
    dns_val: gtk::Label,

    protocol_section: gtk::Box,
    protocol_header: gtk::Label,
    protocol_box: gtk::Box,

    action_btn: gtk::Button,

    current_name: Rc<RefCell<String>>,
    current_uuid: Rc<RefCell<String>>,
    current_active: Rc<RefCell<bool>>,
    on_success: OnSuccessCallback,
}

impl VpnDetailsPage {
    pub fn new(stack: &gtk::Stack) -> Self {
        let root = Box::new(Orientation::Vertical, 12);
        root.add_css_class("network-page");

        let back = Button::with_label("← Back");
        back.add_css_class("back-button");
        back.set_halign(Align::Start);
        back.set_cursor_from_name(Some("pointer"));
        back.connect_clicked(clone![
            #[weak]
            stack,
            move |_| {
                stack.set_visible_child_name("networks");
            }
        ]);
        root.append(&back);

        let header = Box::new(Orientation::Horizontal, 6);
        let icon = Image::from_icon_name("network-vpn-symbolic");
        icon.set_pixel_size(24);

        let title = Label::new(None);
        title.add_css_class("network-title");

        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let forget_btn = Button::with_label("Forget");
        forget_btn.add_css_class("forget-button");
        forget_btn.set_halign(Align::End);
        forget_btn.set_valign(Align::Center);
        forget_btn.set_cursor_from_name(Some("pointer"));

        header.append(&icon);
        header.append(&title);
        header.append(&spacer);
        header.append(&forget_btn);
        root.append(&header);

        // Basic section
        let basic_box = Box::new(Orientation::Vertical, 6);
        basic_box.add_css_class("basic-section");
        let basic_header = Label::new(Some("Basic"));
        basic_header.add_css_class("section-header");
        basic_box.append(&basic_header);

        let status_val = Label::new(None);
        let vpn_type_val = Label::new(None);
        let interface_val = Label::new(None);

        Self::add_row(&basic_box, "Connection Status", &status_val);
        Self::add_row(&basic_box, "VPN Type", &vpn_type_val);
        Self::add_row(&basic_box, "Interface", &interface_val);

        let action_btn = Button::with_label("Connect");
        action_btn.add_css_class("vpn-action-btn");
        action_btn.set_halign(Align::Start);
        action_btn.set_margin_top(8);
        action_btn.set_cursor_from_name(Some("pointer"));
        basic_box.append(&action_btn);

        root.append(&basic_box);

        // Network section (visible only when active)
        let network_section = Box::new(Orientation::Vertical, 6);
        network_section.add_css_class("advanced-section");
        let net_header = Label::new(Some("Network"));
        net_header.add_css_class("section-header");
        network_section.append(&net_header);

        let ip4_val = Label::new(None);
        let ip6_val = Label::new(None);
        let gateway_val = Label::new(None);
        let dns_val = Label::new(None);

        Self::add_row(&network_section, "IPv4 Address", &ip4_val);
        Self::add_row(&network_section, "IPv6 Address", &ip6_val);
        Self::add_row(&network_section, "Gateway", &gateway_val);
        Self::add_row(&network_section, "DNS Servers", &dns_val);

        network_section.set_visible(false);
        root.append(&network_section);

        // Protocol section (dynamic content)
        let protocol_section = Box::new(Orientation::Vertical, 6);
        protocol_section.add_css_class("advanced-section");
        let protocol_header = Label::new(Some("Protocol"));
        protocol_header.add_css_class("section-header");
        protocol_section.append(&protocol_header);

        let protocol_box = Box::new(Orientation::Vertical, 6);
        protocol_section.append(&protocol_box);

        protocol_section.set_visible(false);
        root.append(&protocol_section);

        let current_name = Rc::new(RefCell::new(String::new()));
        let current_uuid = Rc::new(RefCell::new(String::new()));
        let current_active = Rc::new(RefCell::new(false));
        let on_success_callback: OnSuccessCallback = Rc::new(RefCell::new(None));

        // Forget handler
        {
            let stack_clone = stack.clone();
            let name_clone = current_name.clone();
            let on_success_clone = on_success_callback.clone();

            forget_btn.connect_clicked(move |btn| {
                let stack = stack_clone.clone();
                let name = name_clone.borrow().clone();
                let on_success = on_success_clone.clone();
                btn.set_sensitive(false);
                let btn = btn.clone();

                glib::MainContext::default().spawn_local(async move {
                    if let Ok(nm) = NetworkManager::new().await
                        && nm.forget_vpn(&name).await.is_ok()
                    {
                        stack.set_visible_child_name("networks");
                        if let Some(callback) = on_success.borrow().as_ref() {
                            callback();
                        }
                    }
                    btn.set_sensitive(true);
                });
            });
        }

        // Connect/Disconnect handler
        {
            let uuid_clone = current_uuid.clone();
            let active_clone = current_active.clone();
            let name_clone = current_name.clone();
            let on_success_clone = on_success_callback.clone();
            let stack_clone = stack.clone();

            action_btn.connect_clicked(move |btn| {
                let uuid = uuid_clone.borrow().clone();
                let active = *active_clone.borrow();
                let name = name_clone.borrow().clone();
                let on_success = on_success_clone.clone();
                let stack = stack_clone.clone();
                btn.set_sensitive(false);
                let btn = btn.clone();

                glib::MainContext::default().spawn_local(async move {
                    if let Ok(nm) = NetworkManager::new().await {
                        let results = if active {
                            nm.disconnect_vpn_by_uuid(&uuid).await
                        } else {
                            nm.connect_vpn_by_uuid(&uuid).await
                        };

                        match results {
                            Ok(_) => {
                                stack.set_visible_child_name("networks");
                                if let Some(callback) = on_success.borrow().as_ref() {
                                    callback();
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "VPN {} failed for '{}': {}",
                                    if active { "disconnect" } else { "connect" },
                                    name,
                                    e
                                );
                            }
                        }
                    }
                    btn.set_sensitive(true);
                });
            });
        }

        Self {
            root,
            title,
            status_val,
            vpn_type_val,
            interface_val,
            network_section,
            ip4_val,
            ip6_val,
            gateway_val,
            dns_val,
            protocol_section,
            protocol_header,
            protocol_box,
            action_btn,
            current_name,
            current_uuid,
            current_active,
            on_success: on_success_callback,
        }
    }

    pub fn set_on_success(&self, callback: Rc<dyn Fn()>) {
        *self.on_success.borrow_mut() = Some(callback);
    }

    fn add_row(parent: &gtk::Box, key_text: &str, val_widget: &gtk::Label) {
        let row = Box::new(Orientation::Vertical, 3);
        row.set_halign(Align::Start);

        let key = Label::new(Some(key_text));
        key.add_css_class("info-label");
        key.set_halign(Align::Start);

        val_widget.add_css_class("info-value");
        val_widget.set_halign(Align::Start);

        row.append(&key);
        row.append(val_widget);
        parent.append(&row);
    }

    fn clear_protocol_box(&self) {
        let mut child = self.protocol_box.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            self.protocol_box.remove(&widget);
        }
    }

    fn add_protocol_row(&self, key_text: &str, value: &str) {
        let row = Box::new(Orientation::Vertical, 3);
        row.set_halign(Align::Start);

        let key = Label::new(Some(key_text));
        key.add_css_class("info-label");
        key.set_halign(Align::Start);

        let val = Label::new(Some(value));
        val.add_css_class("info-value");
        val.set_halign(Align::Start);

        row.append(&key);
        row.append(&val);
        self.protocol_box.append(&row);
    }

    pub fn update(&self, vpn: &VpnConnection) {
        self.current_name.replace(vpn.name.clone());
        self.current_uuid.replace(vpn.uuid.clone());
        self.current_active.replace(vpn.active);

        self.title.set_text(&vpn.name);
        self.status_val.set_text(if vpn.active {
            "Connected"
        } else {
            "Disconnected"
        });

        let type_label = match &vpn.vpn_type {
            nmrs::VpnType::WireGuard { .. } => "WireGuard",
            nmrs::VpnType::OpenVpn { .. } => "OpenVPN",
            nmrs::VpnType::OpenConnect { .. } => "OpenConnect",
            nmrs::VpnType::StrongSwan { .. } => "strongSwan",
            nmrs::VpnType::Pptp { .. } => "PPTP",
            nmrs::VpnType::L2tp { .. } => "L2TP",
            _ => "VPN",
        };
        self.vpn_type_val.set_text(type_label);

        self.interface_val
            .set_text(vpn.interface.as_deref().unwrap_or("-"));

        if vpn.active {
            self.action_btn.set_label("Disconnect");
            self.action_btn.remove_css_class("vpn-connect-btn");
            self.action_btn.add_css_class("vpn-disconnect-btn");
        } else {
            self.action_btn.set_label("Connect");
            self.action_btn.remove_css_class("vpn-disconnect-btn");
            self.action_btn.add_css_class("vpn-connect-btn");
        }

        // Populate protocol section from VpnType
        self.clear_protocol_box();
        match &vpn.vpn_type {
            nmrs::VpnType::WireGuard {
                peer_public_key,
                endpoint,
                allowed_ips,
                persistent_keepalive,
                ..
            } => {
                self.protocol_header.set_text("WireGuard");
                if let Some(pk) = peer_public_key {
                    self.add_protocol_row("Peer Public Key", pk);
                }
                if let Some(ep) = endpoint {
                    self.add_protocol_row("Endpoint", ep);
                }
                if !allowed_ips.is_empty() {
                    self.add_protocol_row("Allowed IPs", &allowed_ips.join(", "));
                }
                if let Some(ka) = persistent_keepalive {
                    self.add_protocol_row("Keepalive", &format!("{ka}s"));
                }
                self.protocol_section.set_visible(true);
            }
            nmrs::VpnType::OpenVpn {
                remote,
                connection_type,
                user_name,
                ca,
                ..
            } => {
                self.protocol_header.set_text("OpenVPN");
                if let Some(r) = remote {
                    self.add_protocol_row("Remote", r);
                }
                if let Some(ct) = connection_type {
                    self.add_protocol_row("Auth Type", &format!("{ct:?}"));
                }
                if let Some(u) = user_name {
                    self.add_protocol_row("Username", u);
                }
                if let Some(c) = ca {
                    self.add_protocol_row("CA Certificate", c);
                }
                self.protocol_section.set_visible(true);
            }
            _ => {
                self.protocol_section.set_visible(false);
            }
        }

        self.network_section.set_visible(false);
    }

    pub fn enrich_with_info(&self, info: &VpnConnectionInfo) {
        self.ip4_val
            .set_text(info.ip4_address.as_deref().unwrap_or("-"));
        self.ip6_val
            .set_text(info.ip6_address.as_deref().unwrap_or("-"));
        self.gateway_val
            .set_text(info.gateway.as_deref().unwrap_or("-"));

        if info.dns_servers.is_empty() {
            self.dns_val.set_text("-");
        } else {
            self.dns_val.set_text(&info.dns_servers.join(", "));
        }

        self.interface_val
            .set_text(info.interface.as_deref().unwrap_or("-"));

        self.network_section.set_visible(true);

        // Enrich protocol details from active info
        if let Some(details) = &info.details {
            self.clear_protocol_box();
            match details {
                VpnDetails::WireGuard {
                    public_key,
                    endpoint,
                } => {
                    self.protocol_header.set_text("WireGuard");
                    if let Some(pk) = public_key {
                        self.add_protocol_row("Public Key", pk);
                    }
                    if let Some(ep) = endpoint {
                        self.add_protocol_row("Endpoint", ep);
                    }
                    self.protocol_section.set_visible(true);
                }
                VpnDetails::OpenVpn {
                    remote,
                    port,
                    protocol,
                    cipher,
                    auth,
                    compression,
                } => {
                    self.protocol_header.set_text("OpenVPN");
                    self.add_protocol_row("Remote", remote);
                    self.add_protocol_row("Port", &port.to_string());
                    self.add_protocol_row("Protocol", protocol);
                    if let Some(c) = cipher {
                        self.add_protocol_row("Cipher", c);
                    }
                    if let Some(a) = auth {
                        self.add_protocol_row("Auth", a);
                    }
                    if let Some(comp) = compression {
                        self.add_protocol_row("Compression", comp);
                    }
                    self.protocol_section.set_visible(true);
                }
                _ => {}
            }
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }
}
