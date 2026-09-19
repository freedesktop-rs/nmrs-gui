use std::collections::HashSet;
use std::time::Duration;

use futures::StreamExt;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::InputState;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, StyledExt, WindowExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::backend::Backend;
use crate::config::{Config, LayoutMode, Page, Store};
use crate::model::{Action, Reachability, State};
use crate::theme;
use crate::ui::{network, profiles, vpn, wifi};

actions!(
    nmrs,
    [
        Quit, Dismiss, Refresh, NextPage, PrevPage, Page1, Page2, Page3, Page4, Page5, Page6, Page7
    ]
);

const CONTEXT: &str = "Nmrs";

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-q", Quit, Some(CONTEXT)),
        KeyBinding::new("escape", Dismiss, Some(CONTEXT)),
        KeyBinding::new("ctrl-r", Refresh, Some(CONTEXT)),
        KeyBinding::new("f5", Refresh, Some(CONTEXT)),
        KeyBinding::new("ctrl-tab", NextPage, Some(CONTEXT)),
        KeyBinding::new("ctrl-shift-tab", PrevPage, Some(CONTEXT)),
        KeyBinding::new("alt-1", Page1, Some(CONTEXT)),
        KeyBinding::new("alt-2", Page2, Some(CONTEXT)),
        KeyBinding::new("alt-3", Page3, Some(CONTEXT)),
        KeyBinding::new("alt-4", Page4, Some(CONTEXT)),
        KeyBinding::new("alt-5", Page5, Some(CONTEXT)),
        KeyBinding::new("alt-6", Page6, Some(CONTEXT)),
        KeyBinding::new("alt-7", Page7, Some(CONTEXT)),
    ]);
}

pub struct NmApp {
    pub backend: Backend,
    pub store: Store,
    pub config: Config,
    pub state: Option<State>,
    pub load_error: Option<String>,
    pub loading: bool,
    pub page: Page,
    pub pending: HashSet<String>,
    pub confirming: Option<String>,
    pub wifi: wifi::WifiUi,
    pub vpn: vpn::VpnUi,
    pub profiles: profiles::ProfilesUi,
    pub network: network::NetworkUi,
    pub appearance: crate::ui::appearance::AppearanceUi,
    pub subscriptions: Vec<Subscription>,
    focus: FocusHandle,
}

impl NmApp {
    pub fn new(
        backend: Backend,
        store: Store,
        config: Config,
        page: Option<Page>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let page = page
            .filter(|p| config.layout.pages.contains(p))
            .or(config.layout.pages.first().copied())
            .unwrap_or(Page::Wifi);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);

        let mut app = Self {
            wifi: wifi::WifiUi::new(window, cx),
            vpn: vpn::VpnUi::new(window, cx),
            profiles: profiles::ProfilesUi::new(window, cx),
            network: network::NetworkUi::new(window, cx),
            appearance: crate::ui::appearance::AppearanceUi::new(&config, window, cx),
            backend,
            store,
            config,
            state: None,
            load_error: None,
            loading: false,
            page,
            pending: HashSet::new(),
            confirming: None,
            subscriptions: Vec::new(),
            focus,
        };
        app.subscribe_inputs(window, cx);

        cx.observe_window_appearance(window, |this, window, cx| {
            theme::apply(&this.config.appearance, Some(window), cx);
        })
        .detach();

        app.watch_events(window, cx);
        app.watch_files(window, cx);
        app.refresh(cx);
        if app.config.wifi.scan_on_open && !app.backend.demo {
            app.run("scan", Action::Scan, window, cx);
        }
        app
    }

    fn subscribe_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        wifi::subscribe(self, window, cx);
        profiles::subscribe(self, window, cx);
        crate::ui::appearance::subscribe(self, window, cx);
    }

    fn watch_events(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut events = self.backend.watch();
        cx.spawn_in(window, async move |this, cx| {
            while events.next().await.is_some() {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                while events.try_recv().is_ok() {}
                if this.update(cx, |this, cx| this.refresh(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn watch_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let alive = this.update_in(cx, |this, window, cx| {
                    let mut dirty = false;
                    if let Some(errors) = theme::reload_if_changed(cx) {
                        for err in errors {
                            window.push_notification(
                                Notification::error(err).title("Theme failed to load"),
                                cx,
                            );
                        }
                        dirty = true;
                    }
                    if this.store.changed() {
                        match this.store.load() {
                            Ok(config) => {
                                this.appearance.sync(&config, window, cx);
                                this.config = config;
                                dirty = true;
                            }
                            Err(err) => window.push_notification(
                                Notification::error(err).title("Config failed to load"),
                                cx,
                            ),
                        }
                    }
                    if dirty {
                        theme::apply(&this.config.appearance, Some(window), cx);
                        if !this.config.layout.pages.contains(&this.page) {
                            if let Some(first) = this.config.layout.pages.first() {
                                this.page = *first;
                            }
                        }
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        self.loading = true;
        let fut = self.backend.state();
        cx.spawn(async move |this, cx| {
            let result = fut.await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(state) => {
                        this.state = Some(state);
                        this.load_error = None;
                    }
                    Err(err) => this.load_error = Some(format!("{err:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn is_pending(&self, key: &str) -> bool {
        self.pending.contains(key)
    }

    pub fn run(
        &mut self,
        key: impl Into<String>,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_then(key, action, window, cx, |_, _, _| {});
    }

    pub fn run_then(
        &mut self,
        key: impl Into<String>,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let key = key.into();
        if !self.pending.insert(key.clone()) {
            return;
        }
        self.confirming = None;
        let title = action.describe();
        let fut = self.backend.execute(action);
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = fut.await;
            this.update_in(cx, |this, window, cx| {
                this.pending.remove(&key);
                match result {
                    Ok(()) => then(this, window, cx),
                    Err(err) => window.push_notification(
                        Notification::error(format!("{err:#}")).title(title),
                        cx,
                    ),
                }
                this.refresh(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn confirm(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        if !self.config.behavior.confirm_forget || self.confirming.as_deref() == Some(key) {
            self.confirming = None;
            return true;
        }
        self.confirming = Some(key.to_string());
        cx.notify();
        false
    }

    pub fn is_confirming(&self, key: &str) -> bool {
        self.confirming.as_deref() == Some(key)
    }

    pub fn update_config(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&mut Config),
    ) {
        edit(&mut self.config);
        if let Err(err) = self.store.save(&self.config) {
            window.push_notification(Notification::error(err).title("Config not saved"), cx);
        }
        theme::apply(&self.config.appearance, Some(window), cx);
        cx.notify();
    }

    pub fn go(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        if self.page != page {
            self.page = page;
            self.confirming = None;
            if page == Page::Profiles {
                profiles::ensure_selection(self, window, cx);
            }
            cx.notify();
        }
    }

    fn step_page(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let pages = self.config.layout.pages.clone();
        if pages.is_empty() {
            return;
        }
        let idx = pages.iter().position(|p| *p == self.page).unwrap_or(0) as isize;
        let next = (idx + delta).rem_euclid(pages.len() as isize) as usize;
        self.go(pages[next], window, cx);
    }

    fn go_index(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.config.layout.pages.get(index).copied() {
            self.go(page, window, cx);
        }
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dismissed = self.confirming.take().is_some()
            || wifi::dismiss(self, window, cx)
            || vpn::dismiss(self, cx);
        if dismissed {
            window.focus(&self.focus, cx);
            cx.notify();
        } else if self.config.behavior.close_on_escape {
            cx.quit();
        }
    }

    pub fn row_height(&self) -> Pixels {
        px(self.config.appearance.density.row_height())
    }

    pub fn gap(&self) -> Pixels {
        px(self.config.appearance.density.gap())
    }

    pub fn status_line(&self) -> (Reachability, Option<String>) {
        let Some(state) = &self.state else {
            return (Reachability::Unknown, None);
        };
        let primary = state
            .wired
            .iter()
            .find(|d| d.state.is_up())
            .map(|d| d.profile.clone().unwrap_or_else(|| d.interface.clone()))
            .or_else(|| {
                state
                    .networks
                    .iter()
                    .find(|n| n.active)
                    .map(|n| n.ssid.clone())
            });
        (state.connectivity.state, primary)
    }

    fn render_nav_item(&self, page: Page, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let active = self.page == page;
        let item = div()
            .id(SharedString::from(format!("nav-{}", page.label())))
            .h_flex()
            .items_center()
            .gap_2p5()
            .rounded(theme.radius)
            .cursor_pointer()
            .text_sm()
            .when(active, |s| {
                s.bg(theme.sidebar_accent)
                    .text_color(theme.sidebar_accent_foreground)
            })
            .when(!active, |s| {
                s.text_color(theme.muted_foreground)
                    .hover(|s| s.bg(theme.sidebar_accent.opacity(0.5)))
            })
            .on_click(cx.listener(move |this, _, window, cx| this.go(page, window, cx)))
            .child(Icon::new(page.icon()).size_4());
        if compact {
            item.size_9()
                .justify_center()
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(page.label()).build(window, cx)
                })
                .into_any_element()
        } else {
            item.h_8().px_2p5().child(page.label()).into_any_element()
        }
    }

    fn render_status(&self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (reach, primary) = self.status_line();
        let color = match reach {
            Reachability::Full => theme.success,
            Reachability::Limited | Reachability::Portal => theme.warning,
            Reachability::None => theme.danger,
            Reachability::Unknown => theme.muted_foreground,
        };
        let dot = div().size_2().rounded_full().bg(color).flex_shrink_0();
        if compact {
            return div()
                .h_flex()
                .items_center()
                .gap_2()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(dot)
                .child(reach.label())
                .into_any_element();
        }
        div()
            .v_flex()
            .gap_0p5()
            .px_2p5()
            .py_2()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .child(dot)
                    .child(reach.label()),
            )
            .when_some(primary, |s, p| {
                s.child(
                    div()
                        .pl_4()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .truncate()
                        .child(p),
                )
            })
            .into_any_element()
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let pages = self.config.layout.pages.clone();
        div()
            .v_flex()
            .w(px(204.))
            .h_full()
            .flex_shrink_0()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .p_3()
            .gap_0p5()
            .children(
                pages
                    .into_iter()
                    .filter(|p| *p != Page::Appearance)
                    .map(|p| self.render_nav_item(p, false, cx)),
            )
            .child(div().flex_1())
            .when(self.config.layout.pages.contains(&Page::Appearance), |s| {
                s.child(self.render_nav_item(Page::Appearance, false, cx))
            })
            .child(
                div()
                    .mt_2()
                    .pt_2()
                    .border_t_1()
                    .border_color(theme.sidebar_border)
                    .child(self.render_status(false, cx)),
            )
            .into_any_element()
    }

    fn render_topbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let pages = self.config.layout.pages.clone();
        div()
            .h_flex()
            .items_center()
            .gap_1()
            .px_3()
            .py_2()
            .bg(theme.sidebar)
            .border_b_1()
            .border_color(theme.sidebar_border)
            .children(pages.into_iter().map(|p| self.render_nav_item(p, true, cx)))
            .child(div().flex_1())
            .child(self.render_status(true, cx))
            .into_any_element()
    }

    fn render_body(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.state.is_none() {
            let theme = cx.theme();
            let message = self.load_error.clone();
            return div()
                .v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_3()
                .text_sm()
                .text_color(theme.muted_foreground)
                .map(|s| match message {
                    Some(err) => s
                        .child(Icon::new(IconName::Unplug).size_6())
                        .child(div().max_w(px(360.)).text_center().child(err))
                        .child(
                            Button::new("retry")
                                .small()
                                .label("Retry")
                                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                        ),
                    None => s.child(Icon::new(IconName::LoaderCircle).size_5().with_animation(
                        "loading",
                        Animation::new(Duration::from_secs(1)).repeat(),
                        |icon, delta| icon.transform(Transformation::rotate(percentage(delta))),
                    )),
                })
                .into_any_element();
        }
        match self.page {
            Page::Wifi => wifi::render(self, window, cx),
            Page::Ethernet => crate::ui::ethernet::render(self, window, cx),
            Page::Vpn => vpn::render(self, window, cx),
            Page::Bluetooth => crate::ui::bluetooth::render(self, window, cx),
            Page::Profiles => profiles::render(self, window, cx),
            Page::Network => network::render(self, window, cx),
            Page::Appearance => crate::ui::appearance::render(self, window, cx),
        }
    }
}

impl Render for NmApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compact = self.config.layout.mode == LayoutMode::Compact;
        let wide = self.page == Page::Profiles && !compact;
        let pad = if compact { px(16.) } else { px(28.) };
        let body = self.render_body(window, cx);
        let theme = cx.theme();

        let content = if wide {
            div()
                .id("content")
                .flex_1()
                .h_full()
                .min_w_0()
                .child(body)
                .into_any_element()
        } else {
            div()
                .id("content")
                .flex_1()
                .h_full()
                .min_w_0()
                .overflow_y_scrollbar()
                .child(
                    div()
                        .w_full()
                        .max_w(px(780.))
                        .mx_auto()
                        .px(pad)
                        .pt(pad)
                        .pb(px(40.))
                        .child(body),
                )
                .into_any_element()
        };

        div()
            .id("nmrs")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| this.dismiss(window, cx)))
            .on_action(cx.listener(|this, _: &Refresh, window, cx| {
                if this.page == Page::Wifi && !this.backend.demo {
                    this.run("scan", Action::Scan, window, cx);
                }
                this.refresh(cx);
            }))
            .on_action(cx.listener(|this, _: &NextPage, w, cx| this.step_page(1, w, cx)))
            .on_action(cx.listener(|this, _: &PrevPage, w, cx| this.step_page(-1, w, cx)))
            .on_action(cx.listener(|this, _: &Page1, w, cx| this.go_index(0, w, cx)))
            .on_action(cx.listener(|this, _: &Page2, w, cx| this.go_index(1, w, cx)))
            .on_action(cx.listener(|this, _: &Page3, w, cx| this.go_index(2, w, cx)))
            .on_action(cx.listener(|this, _: &Page4, w, cx| this.go_index(3, w, cx)))
            .on_action(cx.listener(|this, _: &Page5, w, cx| this.go_index(4, w, cx)))
            .on_action(cx.listener(|this, _: &Page6, w, cx| this.go_index(5, w, cx)))
            .on_action(cx.listener(|this, _: &Page7, w, cx| this.go_index(6, w, cx)))
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .font_family(theme.font_family.clone())
            .text_size(theme.font_size)
            .map(|s| {
                if compact {
                    s.v_flex().child(self.render_topbar(cx)).child(content)
                } else {
                    s.h_flex().child(self.render_sidebar(cx)).child(content)
                }
            })
    }
}

pub fn input(
    placeholder: &str,
    masked: bool,
    window: &mut Window,
    cx: &mut Context<NmApp>,
) -> Entity<InputState> {
    let placeholder = placeholder.to_string();
    cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder(placeholder)
            .masked(masked)
    })
}

pub fn value(input: &Entity<InputState>, cx: &App) -> String {
    input.read(cx).value().trim().to_string()
}

pub fn list(input: &Entity<InputState>, cx: &App) -> Vec<String> {
    value(input, cx)
        .split([',', ' ', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

pub fn set(input: &Entity<InputState>, text: &str, window: &mut Window, cx: &mut Context<NmApp>) {
    let text = text.to_string();
    input.update(cx, |state, cx| state.set_value(text, window, cx));
}

pub fn ghost(id: impl Into<ElementId>, icon: IconName, tooltip: &'static str) -> Button {
    Button::new(id).ghost().small().icon(icon).tooltip(tooltip)
}
