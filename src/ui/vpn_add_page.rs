use glib::clone;
use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, Entry, FileChooserAction, FileChooserDialog, Label, Orientation,
    ResponseType, Stack,
};
use nmrs::{NetworkManager, WireGuardConfig, WireGuardPeer};
use std::cell::RefCell;
use std::rc::Rc;

type OnSuccessCallback = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

pub struct VpnAddPage {
    root: gtk::Box,
    on_success: OnSuccessCallback,
}

impl VpnAddPage {
    pub fn new(stack: &gtk::Stack, parent_window: &gtk::ApplicationWindow) -> Self {
        let root = GtkBox::new(Orientation::Vertical, 12);
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

        let title = Label::new(Some("Add VPN"));
        title.add_css_class("network-title");
        title.set_halign(Align::Start);
        root.append(&title);

        let tab_stack = Stack::new();
        let on_success: OnSuccessCallback = Rc::new(RefCell::new(None));

        let tab_bar = GtkBox::new(Orientation::Horizontal, 8);
        tab_bar.set_margin_top(4);
        tab_bar.set_margin_bottom(8);

        let wg_tab_btn = Button::with_label("WireGuard");
        wg_tab_btn.add_css_class("vpn-tab-btn");
        wg_tab_btn.add_css_class("vpn-tab-active");
        wg_tab_btn.set_cursor_from_name(Some("pointer"));

        let ovpn_tab_btn = Button::with_label("Import OpenVPN");
        ovpn_tab_btn.add_css_class("vpn-tab-btn");
        ovpn_tab_btn.set_cursor_from_name(Some("pointer"));

        tab_bar.append(&wg_tab_btn);
        tab_bar.append(&ovpn_tab_btn);
        root.append(&tab_bar);

        {
            let tab_stack_c = tab_stack.clone();
            let ovpn_btn_c = ovpn_tab_btn.clone();
            wg_tab_btn.connect_clicked(move |btn| {
                tab_stack_c.set_visible_child_name("wireguard");
                btn.add_css_class("vpn-tab-active");
                ovpn_btn_c.remove_css_class("vpn-tab-active");
            });
        }
        {
            let tab_stack_c = tab_stack.clone();
            let wg_btn_c = wg_tab_btn.clone();
            ovpn_tab_btn.connect_clicked(move |btn| {
                tab_stack_c.set_visible_child_name("openvpn");
                btn.add_css_class("vpn-tab-active");
                wg_btn_c.remove_css_class("vpn-tab-active");
            });
        }

        let wg_page = Self::build_wireguard_tab(stack, &on_success);
        tab_stack.add_named(&wg_page, Some("wireguard"));

        let ovpn_page = Self::build_openvpn_tab(stack, parent_window, &on_success);
        tab_stack.add_named(&ovpn_page, Some("openvpn"));

        tab_stack.set_visible_child_name("wireguard");
        root.append(&tab_stack);

        Self { root, on_success }
    }

    pub fn set_on_success(&self, callback: Rc<dyn Fn()>) {
        *self.on_success.borrow_mut() = Some(callback);
    }

    fn labeled_entry(parent: &gtk::Box, label_text: &str, placeholder: &str) -> Entry {
        let label = Label::new(Some(label_text));
        label.add_css_class("info-label");
        label.set_halign(Align::Start);
        parent.append(&label);

        let entry = Entry::new();
        entry.add_css_class("vpn-entry");
        entry.set_placeholder_text(Some(placeholder));
        parent.append(&entry);
        entry
    }

    fn build_wireguard_tab(stack: &gtk::Stack, on_success: &OnSuccessCallback) -> gtk::Box {
        let page = GtkBox::new(Orientation::Vertical, 8);

        let conn_header = Label::new(Some("Connection"));
        conn_header.add_css_class("section-header");
        page.append(&conn_header);

        let name_entry = Self::labeled_entry(&page, "Name", "e.g. HomeVPN");
        let gateway_entry = Self::labeled_entry(&page, "Gateway", "vpn.example.com:51820");
        let privkey_entry =
            Self::labeled_entry(&page, "Private Key", "Base64 WireGuard private key");
        privkey_entry.set_visibility(false);
        let address_entry = Self::labeled_entry(&page, "Address", "10.0.0.2/24");

        let peer_header = Label::new(Some("Peer"));
        peer_header.add_css_class("section-header");
        peer_header.set_margin_top(12);
        page.append(&peer_header);

        let peer_pubkey = Self::labeled_entry(&page, "Public Key", "Peer's public key");
        let peer_endpoint = Self::labeled_entry(&page, "Endpoint", "vpn.example.com:51820");
        let peer_allowed = Self::labeled_entry(&page, "Allowed IPs", "0.0.0.0/0");
        let peer_keepalive = Self::labeled_entry(&page, "Persistent Keepalive", "25 (optional)");

        let opt_header = Label::new(Some("Optional"));
        opt_header.add_css_class("section-header");
        opt_header.set_margin_top(12);
        page.append(&opt_header);

        let dns_entry = Self::labeled_entry(&page, "DNS Servers", "1.1.1.1, 8.8.8.8 (optional)");
        let mtu_entry = Self::labeled_entry(&page, "MTU", "1420 (optional)");

        let status_label = Label::new(None);
        status_label.add_css_class("vpn-status-label");
        status_label.set_halign(Align::Start);
        status_label.set_margin_top(8);
        page.append(&status_label);

        let connect_btn = Button::with_label("Connect");
        connect_btn.add_css_class("vpn-connect-btn");
        connect_btn.set_halign(Align::Start);
        connect_btn.set_margin_top(8);
        connect_btn.set_cursor_from_name(Some("pointer"));
        page.append(&connect_btn);

        {
            let stack = stack.clone();
            let on_success = on_success.clone();
            let status = status_label.clone();

            connect_btn.connect_clicked(move |btn| {
                let name = name_entry.text().to_string();
                let gateway = gateway_entry.text().to_string();
                let privkey = privkey_entry.text().to_string();
                let address = address_entry.text().to_string();
                let pubkey = peer_pubkey.text().to_string();
                let endpoint = peer_endpoint.text().to_string();
                let allowed = peer_allowed.text().to_string();
                let keepalive = peer_keepalive.text().to_string();
                let dns_text = dns_entry.text().to_string();
                let mtu_text = mtu_entry.text().to_string();

                if name.trim().is_empty()
                    || gateway.trim().is_empty()
                    || privkey.trim().is_empty()
                    || address.trim().is_empty()
                    || pubkey.trim().is_empty()
                    || endpoint.trim().is_empty()
                    || allowed.trim().is_empty()
                {
                    status.set_text("Fill in all required fields");
                    return;
                }

                let stack = stack.clone();
                let on_success = on_success.clone();
                let status = status.clone();
                btn.set_sensitive(false);
                let btn = btn.clone();

                status.set_text("Connecting...");

                glib::MainContext::default().spawn_local(async move {
                    let allowed_ips: Vec<String> = allowed
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    let mut peer = WireGuardPeer::new(pubkey.trim(), endpoint.trim(), allowed_ips);

                    if let Ok(ka) = keepalive.trim().parse::<u32>() {
                        peer = peer.with_persistent_keepalive(ka);
                    }

                    let mut config = WireGuardConfig::new(
                        name.trim(),
                        gateway.trim(),
                        privkey.trim(),
                        address.trim(),
                        vec![peer],
                    );

                    if !dns_text.trim().is_empty() {
                        let dns: Vec<String> = dns_text
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        config = config.with_dns(dns);
                    }

                    if let Ok(mtu) = mtu_text.trim().parse::<u32>() {
                        config = config.with_mtu(mtu);
                    }

                    match NetworkManager::new().await {
                        Ok(nm) => match nm.connect_vpn(config).await {
                            Ok(_) => {
                                status.set_text("Connected!");
                                stack.set_visible_child_name("networks");
                                if let Some(callback) = on_success.borrow().as_ref() {
                                    callback();
                                }
                            }
                            Err(e) => {
                                status.set_text(&format!("Failed: {e}"));
                            }
                        },
                        Err(e) => {
                            status.set_text(&format!("NM error: {e}"));
                        }
                    }
                    btn.set_sensitive(true);
                });
            });
        }

        page
    }

    fn build_openvpn_tab(
        stack: &gtk::Stack,
        parent_window: &gtk::ApplicationWindow,
        on_success: &OnSuccessCallback,
    ) -> gtk::Box {
        let page = GtkBox::new(Orientation::Vertical, 8);

        let file_header = Label::new(Some("Configuration File"));
        file_header.add_css_class("section-header");
        page.append(&file_header);

        let file_label = Label::new(Some("OpenVPN File (.ovpn)"));
        file_label.add_css_class("info-label");
        file_label.set_halign(Align::Start);
        page.append(&file_label);

        let file_hbox = GtkBox::new(Orientation::Horizontal, 8);
        let file_entry = Entry::new();
        file_entry.add_css_class("vpn-entry");
        file_entry.set_placeholder_text(Some("/path/to/config.ovpn"));
        file_entry.set_hexpand(true);

        let browse_btn = Button::with_label("Browse...");
        browse_btn.add_css_class("vpn-browse-btn");
        browse_btn.set_cursor_from_name(Some("pointer"));
        file_hbox.append(&file_entry);
        file_hbox.append(&browse_btn);
        page.append(&file_hbox);

        {
            let file_entry_c = file_entry.clone();
            let parent_weak = parent_window.downgrade();
            browse_btn.connect_clicked(move |_| {
                let Some(parent) = parent_weak.upgrade() else {
                    return;
                };
                let file_entry = file_entry_c.clone();
                let dialog = FileChooserDialog::new(
                    Some("Select OpenVPN Configuration"),
                    Some(&parent),
                    FileChooserAction::Open,
                    &[
                        ("Cancel", ResponseType::Cancel),
                        ("Open", ResponseType::Accept),
                    ],
                );
                dialog.connect_response(move |dialog, response| {
                    if response == ResponseType::Accept
                        && let Some(file) = dialog.file()
                        && let Some(path) = file.path()
                    {
                        file_entry.set_text(&path.to_string_lossy());
                    }
                    dialog.close();
                });
                dialog.show();
            });
        }

        let cred_header = Label::new(Some("Credentials (optional)"));
        cred_header.add_css_class("section-header");
        cred_header.set_margin_top(12);
        page.append(&cred_header);

        let username_entry = Self::labeled_entry(&page, "Username", "VPN username (if required)");
        let password_entry = Self::labeled_entry(&page, "Password", "VPN password (if required)");
        password_entry.set_visibility(false);

        let status_label = Label::new(None);
        status_label.add_css_class("vpn-status-label");
        status_label.set_halign(Align::Start);
        status_label.set_margin_top(8);
        page.append(&status_label);

        let import_btn = Button::with_label("Import & Connect");
        import_btn.add_css_class("vpn-connect-btn");
        import_btn.set_halign(Align::Start);
        import_btn.set_margin_top(8);
        import_btn.set_cursor_from_name(Some("pointer"));
        page.append(&import_btn);

        {
            let stack = stack.clone();
            let on_success = on_success.clone();
            let status = status_label.clone();

            import_btn.connect_clicked(move |btn| {
                let path = file_entry.text().to_string();
                let user = username_entry.text().to_string();
                let pass = password_entry.text().to_string();

                if path.trim().is_empty() {
                    status.set_text("Select an .ovpn file");
                    return;
                }

                let stack = stack.clone();
                let on_success = on_success.clone();
                let status = status.clone();
                btn.set_sensitive(false);
                let btn = btn.clone();

                status.set_text("Importing...");

                glib::MainContext::default().spawn_local(async move {
                    let user_opt = if user.trim().is_empty() {
                        None
                    } else {
                        Some(user.as_str())
                    };
                    let pass_opt = if pass.trim().is_empty() {
                        None
                    } else {
                        Some(pass.as_str())
                    };

                    match NetworkManager::new().await {
                        Ok(nm) => match nm.import_ovpn(&path, user_opt, pass_opt).await {
                            Ok(_) => {
                                status.set_text("Imported & Connected!");
                                stack.set_visible_child_name("networks");
                                if let Some(callback) = on_success.borrow().as_ref() {
                                    callback();
                                }
                            }
                            Err(e) => {
                                status.set_text(&format!("Failed: {e}"));
                            }
                        },
                        Err(e) => {
                            status.set_text(&format!("NM error: {e}"));
                        }
                    }
                    btn.set_sensitive(true);
                });
            });
        }

        page
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }
}
