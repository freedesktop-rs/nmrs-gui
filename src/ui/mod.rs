pub mod connect;
pub mod header;
pub mod network_page;
pub mod networks;
pub mod settings_page;
pub mod vpn_add_page;
pub mod vpn_details_page;
pub mod vpn_list;
pub mod wired_devices;
pub mod wired_page;

use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Image, Label, Orientation, ScrolledWindow,
    Spinner, Stack, pango::EllipsizeMode,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Notify;

type Callback = Rc<dyn Fn()>;
type CallbackCell = Rc<std::cell::RefCell<Option<Callback>>>;

const COLOR_SCHEME_PROPERTY: &str = "gtk-interface-color-scheme";

// Read the GTK 4.20 property dynamically for compatibility with older releases
fn system_prefers_dark(settings: &gtk::Settings) -> Option<bool> {
    if !settings.has_property(COLOR_SCHEME_PROPERTY) {
        return None;
    }

    // Treat unknown values as GTK's default light scheme
    glib::EnumValue::from_value(&settings.property_value(COLOR_SCHEME_PROPERTY)).and_then(
        |(_, value)| match value.nick() {
            "unsupported" => None,
            "dark" => Some(true),
            _ => Some(false),
        },
    )
}

fn system_color_scheme() -> Option<(gtk::Settings, bool)> {
    let settings = gtk::Settings::default()?;
    system_prefers_dark(&settings).map(|prefers_dark| (settings, prefers_dark))
}

fn set_interface_color_scheme(settings: &gtk::Settings, prefers_dark: bool) {
    settings.set_gtk_application_prefer_dark_theme(prefers_dark);

    let Some(property) = settings.find_property(COLOR_SCHEME_PROPERTY) else {
        return;
    };
    let Some(value) = glib::EnumClass::with_type(property.value_type())
        .and_then(|class| class.to_value_by_nick(if prefers_dark { "dark" } else { "light" }))
    else {
        return;
    };

    settings.set_property_from_value(COLOR_SCHEME_PROPERTY, &value);
}

fn update_color_scheme(window: &impl IsA<gtk::Widget>, prefers_dark: bool) {
    window.remove_css_class("dark-theme");
    window.remove_css_class("light-theme");
    window.add_css_class(if prefers_dark {
        "dark-theme"
    } else {
        "light-theme"
    });
}

pub(crate) fn inherit_color_scheme(window: &impl IsA<gtk::Widget>, parent: &impl IsA<gtk::Widget>) {
    update_color_scheme(window, parent.has_css_class("dark-theme"));
}

fn sync_system_color_scheme(window: &ApplicationWindow) {
    let Some((settings, prefers_dark)) = system_color_scheme() else {
        return;
    };

    settings.set_gtk_application_prefer_dark_theme(prefers_dark);
    update_color_scheme(window, prefers_dark);
}

pub(crate) fn supports_system_color_scheme() -> bool {
    system_color_scheme().is_some()
}

pub(crate) fn apply_color_scheme_override(window: &ApplicationWindow, prefers_dark: bool) {
    if let Some(settings) = gtk::Settings::default() {
        set_interface_color_scheme(&settings, prefers_dark);
    }

    update_color_scheme(window, prefers_dark);
}

pub(crate) fn apply_system_color_scheme(window: &ApplicationWindow) {
    if let Some(settings) = gtk::Settings::default()
        && settings.has_property(COLOR_SCHEME_PROPERTY)
    {
        settings.reset_property(COLOR_SCHEME_PROPERTY);
    }

    sync_system_color_scheme(window);
}

pub fn freq_to_band(freq: u32) -> Option<&'static str> {
    match freq {
        2400..=2500 => Some("2.4GHz"),
        5150..=5925 => Some("5GHz"),
        5926..=7125 => Some("6GHz"),
        _ => None,
    }
}

pub fn build_ui(app: &Application) {
    let win = ApplicationWindow::new(app);
    win.set_title(Some(""));
    win.set_default_size(450, 600);

    // Preserve the dark default when system preferences are unavailable
    if let Some((settings, prefers_dark)) = system_color_scheme() {
        win.add_css_class("system-theme");
        update_color_scheme(&win, prefers_dark);

        let win_weak = win.downgrade();
        settings.connect_notify_local(Some(COLOR_SCHEME_PROPERTY), move |_, _| {
            if let Some(window) = win_weak.upgrade()
                && window.has_css_class("system-theme")
            {
                sync_system_color_scheme(&window);
            }
        });
    } else {
        apply_color_scheme_override(&win, true);
    }

    let vbox = GtkBox::new(Orientation::Vertical, 0);
    let status = Label::new(None);
    status.set_xalign(0.0);
    status.set_ellipsize(EllipsizeMode::End);
    status.set_max_width_chars(36);

    let conn_icon = Image::from_icon_name("network-offline-symbolic");
    conn_icon.add_css_class("conn-status-icon");

    let conn_name = Label::new(Some("Disconnected"));
    conn_name.set_ellipsize(EllipsizeMode::End);
    conn_name.set_max_width_chars(20);
    conn_name.add_css_class("conn-status-name");

    let scan_spinner = Spinner::new();
    scan_spinner.set_size_request(14, 14);
    scan_spinner.add_css_class("scan-spinner");
    scan_spinner.set_visible(false);

    let list_container = GtkBox::new(Orientation::Vertical, 0);
    let stack = Stack::new();
    let is_scanning = Rc::new(Cell::new(false));

    let spinner = Spinner::new();
    spinner.set_halign(gtk::Align::Center);
    spinner.set_valign(gtk::Align::Center);
    spinner.set_property("width-request", 24i32);
    spinner.set_property("height-request", 24i32);
    spinner.add_css_class("loading-spinner");
    spinner.start();

    stack.add_named(&spinner, Some("loading"));
    stack.set_visible_child_name("loading");

    let status_clone = status.clone();
    let list_container_clone = list_container.clone();
    let stack_clone = stack.clone();
    let win_clone = win.clone();
    let is_scanning_clone = is_scanning.clone();
    let vbox_clone = vbox.clone();

    glib::MainContext::default().spawn_local(async move {
        match nmrs::NetworkManager::new().await {
            Ok(nm) => {
                let nm = Rc::new(nm);

                let details_page = Rc::new(network_page::NetworkPage::new(&stack_clone));
                let details_scroller = ScrolledWindow::new();
                details_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                details_scroller.set_child(Some(details_page.widget()));
                stack_clone.add_named(&details_scroller, Some("details"));

                let wired_details_page = Rc::new(wired_page::WiredPage::new(&stack_clone));
                let wired_details_scroller = ScrolledWindow::new();
                wired_details_scroller
                    .set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                wired_details_scroller.set_child(Some(wired_details_page.widget()));
                stack_clone.add_named(&wired_details_scroller, Some("wired-details"));

                let vpn_details_page = Rc::new(vpn_details_page::VpnDetailsPage::new(&stack_clone));
                let vpn_details_scroller = ScrolledWindow::new();
                vpn_details_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                vpn_details_scroller.set_child(Some(vpn_details_page.widget()));
                stack_clone.add_named(&vpn_details_scroller, Some("vpn-details"));

                let vpn_add = vpn_add_page::VpnAddPage::new(&stack_clone, &win_clone);
                let vpn_add_scroller = ScrolledWindow::new();
                vpn_add_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                vpn_add_scroller.set_child(Some(vpn_add.widget()));
                stack_clone.add_named(&vpn_add_scroller, Some("vpn-add"));

                let settings = settings_page::SettingsPage::new(&stack_clone, &win_clone);
                let settings_scroller = ScrolledWindow::new();
                settings_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
                settings_scroller.set_child(Some(settings.widget()));
                stack_clone.add_named(&settings_scroller, Some("settings"));

                let conn_icon_clone = conn_icon.clone();
                let conn_name_clone = conn_name.clone();
                let scan_spinner_clone = scan_spinner.clone();

                let on_success: Rc<dyn Fn()> = {
                    let list_container = list_container_clone.clone();
                    let is_scanning = is_scanning_clone.clone();
                    let nm = nm.clone();
                    let status = status_clone.clone();
                    let conn_icon = conn_icon_clone.clone();
                    let conn_name = conn_name_clone.clone();
                    let scan_spinner = scan_spinner_clone.clone();
                    let stack = stack_clone.clone();
                    let parent_window = win_clone.clone();
                    let details_page = details_page.clone();
                    let wired_details_page = wired_details_page.clone();
                    let vpn_details_page = vpn_details_page.clone();

                    let on_success_cell: CallbackCell = Rc::new(std::cell::RefCell::new(None));
                    let on_success_cell_clone = on_success_cell.clone();

                    let callback = Rc::new(move || {
                        let list_container = list_container.clone();
                        let is_scanning = is_scanning.clone();
                        let nm = nm.clone();
                        let status = status.clone();
                        let conn_icon = conn_icon.clone();
                        let conn_name = conn_name.clone();
                        let scan_spinner = scan_spinner.clone();
                        let stack = stack.clone();
                        let parent_window = parent_window.clone();
                        let on_success_cell = on_success_cell.clone();
                        let details_page = details_page.clone();
                        let wired_details_page = wired_details_page.clone();
                        let vpn_details_page = vpn_details_page.clone();

                        glib::MainContext::default().spawn_local(async move {
                            let callback = on_success_cell.borrow().as_ref().map(|cb| cb.clone());
                            let refresh_ctx = Rc::new(networks::NetworksContext {
                                nm,
                                on_success: callback.unwrap_or_else(|| Rc::new(|| {})),
                                status,
                                conn_icon,
                                conn_name,
                                scan_spinner,
                                stack,
                                parent_window,
                                details_page: details_page.clone(),
                                wired_details_page: wired_details_page.clone(),
                                vpn_details_page: vpn_details_page.clone(),
                            });
                            header::refresh_networks(refresh_ctx, &list_container, &is_scanning)
                                .await;
                        });
                    }) as Rc<dyn Fn()>;

                    *on_success_cell_clone.borrow_mut() = Some(callback.clone());

                    callback
                };

                let ctx = Rc::new(networks::NetworksContext {
                    nm: nm.clone(),
                    on_success: on_success.clone(),
                    status: status_clone.clone(),
                    conn_icon: conn_icon_clone.clone(),
                    conn_name: conn_name_clone.clone(),
                    scan_spinner: scan_spinner_clone.clone(),
                    stack: stack_clone.clone(),
                    parent_window: win_clone.clone(),
                    details_page: details_page.clone(),
                    wired_details_page,
                    vpn_details_page: vpn_details_page.clone(),
                });

                details_page.set_on_success(on_success.clone());
                vpn_details_page.set_on_success(on_success.clone());
                vpn_add.set_on_success(on_success);

                let header = header::build_header(
                    ctx.clone(),
                    &list_container_clone,
                    is_scanning_clone.clone(),
                );
                vbox_clone.prepend(&header);

                {
                    let nm_device_monitor = nm.clone();
                    let device_notify = Arc::new(Notify::new());

                    let notify_clone = device_notify.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            let notify = notify_clone.clone();
                            let result = nm_device_monitor
                                .monitor_device_changes(move || {
                                    notify.notify_one();
                                })
                                .await;

                            if let Err(e) = result {
                                eprintln!("Device monitoring error: {}, restarting in 5s...", e)
                            }
                            glib::timeout_future_seconds(5).await;
                        }
                    });

                    let list_container_device = list_container_clone.clone();
                    let is_scanning_device = is_scanning_clone.clone();
                    let ctx_device = ctx.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            device_notify.notified().await;
                            glib::timeout_future_seconds(3).await;

                            let current_page = ctx_device.stack.visible_child_name();
                            let on_networks_page = current_page.as_deref() == Some("networks");

                            if !is_scanning_device.get() && on_networks_page {
                                header::refresh_networks_no_scan(
                                    ctx_device.clone(),
                                    &list_container_device,
                                    &is_scanning_device,
                                )
                                .await;
                            }
                        }
                    });
                }

                {
                    let nm_network_monitor = nm.clone();
                    let network_notify = Arc::new(Notify::new());

                    let notify_clone = network_notify.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            let notify = notify_clone.clone();
                            let result = nm_network_monitor
                                .monitor_network_changes(move || {
                                    notify.notify_one();
                                })
                                .await;

                            if let Err(e) = result {
                                eprintln!("Network monitoring error: {}, restarting in 5s...", e)
                            }
                            glib::timeout_future_seconds(5).await;
                        }
                    });

                    let list_container_network = list_container_clone.clone();
                    let is_scanning_network = is_scanning_clone.clone();
                    let ctx_network = ctx.clone();
                    glib::MainContext::default().spawn_local(async move {
                        loop {
                            network_notify.notified().await;
                            glib::timeout_future_seconds(8).await;

                            let current_page = ctx_network.stack.visible_child_name();
                            let on_networks_page = current_page.as_deref() == Some("networks");

                            if !is_scanning_network.get() && on_networks_page {
                                header::refresh_networks_no_scan(
                                    ctx_network.clone(),
                                    &list_container_network,
                                    &is_scanning_network,
                                )
                                .await;
                            }
                        }
                    });
                }
            }
            Err(err) => {
                status_clone.set_text(&format!("Failed to initialize: {err}"));
            }
        }
    });

    let networks_scroller = ScrolledWindow::new();
    networks_scroller.set_vexpand(true);
    networks_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    networks_scroller.set_child(Some(&list_container));

    stack.add_named(&networks_scroller, Some("networks"));

    stack.set_vexpand(true);
    vbox.append(&stack);

    win.set_child(Some(&vbox));
    win.show();
}
