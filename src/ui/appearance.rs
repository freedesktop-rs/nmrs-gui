use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, StyledExt, try_parse_color};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::{NmApp, input, set, value};
use crate::config::{Config, Density, LayoutMode, Page, SignalStyle, Sort};
use crate::theme;
use crate::ui::kit;

const ACCENTS: [&str; 8] = [
    "#7aa2ff", "#3584e4", "#2ec27e", "#e5a50a", "#ff7800", "#e01b24", "#c061cb", "#cba6f7",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    System,
    Fixed,
}

pub struct AppearanceUi {
    accent: Entity<InputState>,
    font: Entity<InputState>,
}

impl AppearanceUi {
    pub fn new(config: &Config, window: &mut Window, cx: &mut Context<NmApp>) -> Self {
        let accent = input("#7aa2ff", false, window, cx);
        let font = input("System default", false, window, cx);
        let this = Self { accent, font };
        this.sync(config, window, cx);
        this
    }

    pub fn sync(&self, config: &Config, window: &mut Window, cx: &mut Context<NmApp>) {
        set(
            &self.accent,
            config.appearance.accent.as_deref().unwrap_or_default(),
            window,
            cx,
        );
        set(
            &self.font,
            config.appearance.font_family.as_deref().unwrap_or_default(),
            window,
            cx,
        );
    }
}

pub fn subscribe(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let accent = app.appearance.accent.clone();
    let font = app.appearance.font.clone();
    app.subscriptions.push(cx.subscribe_in(
        &accent,
        window,
        |this, input, ev: &InputEvent, window, cx| {
            if let InputEvent::Change = ev {
                let text = value(input, cx);
                if text.is_empty() {
                    this.update_config(window, cx, |c| c.appearance.accent = None);
                } else if text.starts_with('#') && try_parse_color(&text).is_ok() {
                    this.update_config(window, cx, |c| c.appearance.accent = Some(text));
                }
            }
        },
    ));
    app.subscriptions.push(cx.subscribe_in(
        &font,
        window,
        |this, input, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                let text = value(input, cx);
                let family = Some(text).filter(|s| !s.is_empty());
                if family != this.config.appearance.font_family {
                    this.update_config(window, cx, |c| c.appearance.font_family = family);
                }
            }
        },
    ));
}

fn theme_card(
    name: SharedString,
    selected: bool,
    on_pick: impl Fn(&mut NmApp, SharedString, &mut Window, &mut Context<NmApp>) + 'static,
    cx: &mut Context<NmApp>,
) -> Stateful<Div> {
    let theme = cx.theme();
    let [bg, side, fg, accent] = theme::swatch(&name, cx).unwrap_or([
        theme.background,
        theme.sidebar,
        theme.foreground,
        theme.primary,
    ]);
    let radius = theme.radius;
    let label = name.clone();
    div()
        .id(SharedString::from(format!("theme-{name}")))
        .v_flex()
        .gap_2()
        .p_1p5()
        .rounded(theme.radius_lg)
        .border_1()
        .cursor_pointer()
        .border_color(if selected {
            theme.primary
        } else {
            theme.border
        })
        .when(selected, |s| s.bg(theme.primary.opacity(0.08)))
        .when(!selected, |s| s.hover(|s| s.bg(theme.accent.opacity(0.5))))
        .on_click(cx.listener(move |this, _, window, cx| on_pick(this, name.clone(), window, cx)))
        .child(
            div()
                .h_flex()
                .h_16()
                .rounded(radius)
                .overflow_hidden()
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .w_5()
                        .h_full()
                        .bg(side)
                        .v_flex()
                        .gap_1()
                        .pt_2()
                        .items_center()
                        .child(div().w_2p5().h_1().rounded_sm().bg(accent))
                        .child(div().w_2p5().h_1().rounded_sm().bg(fg.opacity(0.3)))
                        .child(div().w_2p5().h_1().rounded_sm().bg(fg.opacity(0.3))),
                )
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .bg(bg)
                        .v_flex()
                        .gap_1p5()
                        .p_2()
                        .child(div().w_12().h_1p5().rounded_sm().bg(fg.opacity(0.85)))
                        .child(div().w_16().h_1().rounded_sm().bg(fg.opacity(0.3)))
                        .child(
                            div()
                                .h_flex()
                                .gap_1()
                                .mt_auto()
                                .child(div().w_6().h_2p5().rounded_sm().bg(accent))
                                .child(div().w_6().h_2p5().rounded_sm().bg(fg.opacity(0.15))),
                        ),
                ),
        )
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_1p5()
                .px_0p5()
                .text_xs()
                .child(div().flex_1().truncate().child(label))
                .when(selected, |s| {
                    s.child(
                        Icon::new(IconName::Check)
                            .size_3()
                            .text_color(theme.primary),
                    )
                }),
        )
}

fn grid(cards: Vec<Stateful<Div>>) -> Div {
    div().grid().grid_cols(3).gap_2p5().children(cards)
}

fn stepper(
    id: &'static str,
    label: String,
    on_step: impl Fn(&mut NmApp, i32, &mut Window, &mut Context<NmApp>) + Clone + 'static,
    cx: &mut Context<NmApp>,
) -> Div {
    let theme = cx.theme();
    let dec = on_step.clone();
    div()
        .h_flex()
        .items_center()
        .gap_1()
        .child(
            Button::new(SharedString::from(format!("{id}-dec")))
                .ghost()
                .xsmall()
                .icon(IconName::Minus)
                .on_click(cx.listener(move |this, _, window, cx| dec(this, -1, window, cx))),
        )
        .child(
            div()
                .w_12()
                .text_center()
                .text_sm()
                .text_color(theme.foreground)
                .child(label),
        )
        .child(
            Button::new(SharedString::from(format!("{id}-inc")))
                .ghost()
                .xsmall()
                .icon(IconName::Plus)
                .on_click(cx.listener(move |this, _, window, cx| on_step(this, 1, window, cx))),
        )
}

fn toggle(
    id: &'static str,
    on: bool,
    cx: &mut Context<NmApp>,
    apply: fn(&mut Config, bool),
) -> Switch {
    Switch::new(id)
        .checked(on)
        .on_click(cx.listener(move |this, v: &bool, window, cx| {
            let v = *v;
            this.update_config(window, cx, |c| apply(c, v));
        }))
}

pub fn render(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) -> AnyElement {
    let cfg = app.config.clone();
    let a = &cfg.appearance;
    let mode = if a.theme == "system" {
        Mode::System
    } else {
        Mode::Fixed
    };
    let names = theme::names(cx);
    let dark_now = matches!(
        window.appearance(),
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    );

    let mode_switch = kit::segmented(
        "theme-mode",
        &[(Mode::System, "Follow system"), (Mode::Fixed, "Fixed")],
        mode,
        cx,
        move |this, m, window, cx| {
            let current = theme::resolve_name(&this.config.appearance, dark_now);
            this.update_config(window, cx, |c| {
                c.appearance.theme = match m {
                    Mode::System => "system".into(),
                    Mode::Fixed => current,
                }
            });
        },
    );

    let mut themes = div().v_flex().gap_3();
    match mode {
        Mode::System => {
            let light: Vec<_> = names
                .iter()
                .filter(|(_, d)| !d)
                .map(|(n, _)| {
                    theme_card(
                        n.clone(),
                        *n == a.light_theme,
                        |this, n, window, cx| {
                            this.update_config(window, cx, |c| c.appearance.light_theme = n.into())
                        },
                        cx,
                    )
                })
                .collect();
            let dark: Vec<_> = names
                .iter()
                .filter(|(_, d)| *d)
                .map(|(n, _)| {
                    theme_card(
                        n.clone(),
                        *n == a.dark_theme,
                        |this, n, window, cx| {
                            this.update_config(window, cx, |c| c.appearance.dark_theme = n.into())
                        },
                        cx,
                    )
                })
                .collect();
            let muted = cx.theme().muted_foreground;
            themes = themes
                .child(div().text_xs().text_color(muted).child("Light"))
                .child(grid(light))
                .child(div().pt_2().text_xs().text_color(muted).child("Dark"))
                .child(grid(dark));
        }
        Mode::Fixed => {
            let cards: Vec<_> = names
                .iter()
                .map(|(n, _)| {
                    theme_card(
                        n.clone(),
                        *n == a.theme,
                        |this, n, window, cx| {
                            this.update_config(window, cx, |c| c.appearance.theme = n.into())
                        },
                        cx,
                    )
                })
                .collect();
            themes = themes.child(grid(cards));
        }
    }

    let theme = cx.theme();
    let current_accent = a.accent.clone();
    let accent_row = div()
        .h_flex()
        .items_center()
        .gap_2()
        .px_3p5()
        .py_3()
        .child(
            div()
                .id("accent-default")
                .size_6()
                .rounded_full()
                .border_1()
                .border_color(theme.border)
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .when(current_accent.is_none(), |s| {
                    s.border_2().border_color(theme.foreground)
                })
                .child(
                    Icon::new(IconName::Ban)
                        .size_3()
                        .text_color(theme.muted_foreground),
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    let input = this.appearance.accent.clone();
                    set(&input, "", window, cx);
                    this.update_config(window, cx, |c| c.appearance.accent = None);
                })),
        )
        .children(ACCENTS.iter().map(|hex| {
            let color = try_parse_color(hex).unwrap_or(theme.primary);
            let picked = current_accent
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case(hex));
            let hex = hex.to_string();
            div()
                .id(SharedString::from(format!("accent-{hex}")))
                .size_6()
                .rounded_full()
                .cursor_pointer()
                .bg(color)
                .when(picked, |s| s.border_2().border_color(theme.foreground))
                .on_click(cx.listener(move |this, _, window, cx| {
                    let input = this.appearance.accent.clone();
                    set(&input, &hex, window, cx);
                    let hex = hex.clone();
                    this.update_config(window, cx, |c| c.appearance.accent = Some(hex));
                }))
        }))
        .child(div().flex_1())
        .child(
            div()
                .w(px(120.))
                .child(Input::new(&app.appearance.accent).small()),
        );

    let font_size = a.font_size;
    let radius = a.radius;
    let opacity = a.opacity;

    let style = kit::card(cx)
        .child(kit::setting_row(
            "Font",
            None,
            div()
                .w(px(200.))
                .child(Input::new(&app.appearance.font).small()),
            0,
            cx,
        ))
        .child(kit::setting_row(
            "Text size",
            None,
            stepper(
                "font-size",
                format!("{font_size:.0}"),
                |this, d, window, cx| {
                    this.update_config(window, cx, |c| {
                        c.appearance.font_size =
                            (c.appearance.font_size + d as f32).clamp(10.0, 24.0)
                    })
                },
                cx,
            ),
            1,
            cx,
        ))
        .child(kit::setting_row(
            "Corner radius",
            None,
            stepper(
                "radius",
                format!("{radius:.0}"),
                |this, d, window, cx| {
                    this.update_config(window, cx, |c| {
                        c.appearance.radius =
                            (c.appearance.radius + 2.0 * d as f32).clamp(0.0, 20.0)
                    })
                },
                cx,
            ),
            2,
            cx,
        ))
        .child(kit::setting_row(
            "Background opacity",
            Some(
                "Takes effect with a compositor that blurs or shows through transparent windows"
                    .into(),
            ),
            stepper(
                "opacity",
                format!("{:.0}%", opacity * 100.0),
                |this, d, window, cx| {
                    this.update_config(window, cx, |c| {
                        c.appearance.opacity = ((c.appearance.opacity * 20.0).round() + d as f32)
                            .clamp(6.0, 20.0)
                            / 20.0
                    })
                },
                cx,
            ),
            3,
            cx,
        ))
        .child(kit::setting_row(
            "Density",
            None,
            kit::segmented(
                "density",
                &Density::ALL.map(|d| (d, d.label())),
                a.density,
                cx,
                |this, d, window, cx| this.update_config(window, cx, |c| c.appearance.density = d),
            ),
            4,
            cx,
        ))
        .child(kit::setting_row(
            "Layout",
            None,
            kit::segmented(
                "layout",
                &[LayoutMode::Sidebar, LayoutMode::Compact].map(|m| (m, m.label())),
                cfg.layout.mode,
                cx,
                |this, m, window, cx| this.update_config(window, cx, |c| c.layout.mode = m),
            ),
            5,
            cx,
        ))
        .when(app.compositor.is_some(), |s| {
            s.child(kit::setting_row(
                "Floating window",
                None,
                Switch::new("floating")
                    .checked(cfg.layout.floating.unwrap_or(false))
                    .on_click(cx.listener(|this, on: &bool, window, cx| {
                        let on = *on;
                        this.update_config(window, cx, |c| c.layout.floating = Some(on));
                        this.apply_floating(on, std::time::Duration::ZERO, window, cx);
                    })),
                6,
                cx,
            ))
        });

    let min = cfg.wifi.min_strength;
    let wifi = kit::card(cx)
        .child(kit::setting_row(
            "Signal",
            None,
            kit::segmented(
                "signal",
                &SignalStyle::ALL.map(|s| (s, s.label())),
                cfg.wifi.signal,
                cx,
                |this, s, window, cx| this.update_config(window, cx, |c| c.wifi.signal = s),
            ),
            0,
            cx,
        ))
        .child(kit::setting_row(
            "Sort by",
            None,
            kit::segmented(
                "sort",
                &[Sort::Signal, Sort::Name].map(|s| (s, s.label())),
                cfg.wifi.sort,
                cx,
                |this, s, window, cx| this.update_config(window, cx, |c| c.wifi.sort = s),
            ),
            1,
            cx,
        ))
        .child(kit::setting_row(
            "Hide networks weaker than",
            None,
            stepper(
                "min-strength",
                format!("{min}%"),
                |this, d, window, cx| {
                    this.update_config(window, cx, |c| {
                        c.wifi.min_strength =
                            (c.wifi.min_strength as i32 + 10 * d).clamp(0, 90) as u8
                    })
                },
                cx,
            ),
            2,
            cx,
        ))
        .child(kit::setting_row(
            "Show security",
            None,
            toggle("show-sec", cfg.wifi.show_security, cx, |c, v| {
                c.wifi.show_security = v
            }),
            3,
            cx,
        ))
        .child(kit::setting_row(
            "Show band",
            None,
            toggle("show-band", cfg.wifi.show_band, cx, |c, v| {
                c.wifi.show_band = v
            }),
            4,
            cx,
        ))
        .child(kit::setting_row(
            "Show BSSID",
            None,
            toggle("show-bssid", cfg.wifi.show_bssid, cx, |c, v| {
                c.wifi.show_bssid = v
            }),
            5,
            cx,
        ))
        .child(kit::setting_row(
            "Scan when opened",
            None,
            toggle("scan-open", cfg.wifi.scan_on_open, cx, |c, v| {
                c.wifi.scan_on_open = v
            }),
            6,
            cx,
        ));

    let devices = kit::card(cx).child(kit::setting_row(
        "Show virtual interfaces",
        Some("veth, bridge and tunnel devices".into()),
        toggle("show-virtual", cfg.devices.show_virtual, cx, |c, v| {
            c.devices.show_virtual = v
        }),
        0,
        cx,
    ));

    let behavior = kit::card(cx)
        .child(kit::setting_row(
            "Close after connecting",
            None,
            toggle(
                "close-connect",
                cfg.behavior.close_on_connect,
                cx,
                |c, v| c.behavior.close_on_connect = v,
            ),
            0,
            cx,
        ))
        .child(kit::setting_row(
            "Close with Escape",
            None,
            toggle("close-esc", cfg.behavior.close_on_escape, cx, |c, v| {
                c.behavior.close_on_escape = v
            }),
            1,
            cx,
        ))
        .child(kit::setting_row(
            "Confirm before forgetting",
            None,
            toggle("confirm", cfg.behavior.confirm_forget, cx, |c, v| {
                c.behavior.confirm_forget = v
            }),
            2,
            cx,
        ));

    let pages = kit::card(cx).children(
        Page::ALL
            .iter()
            .filter(|p| **p != Page::Appearance)
            .enumerate()
            .map(|(i, page)| {
                let page = *page;
                let shown = cfg.layout.pages.contains(&page);
                kit::setting_row(
                    page.label(),
                    None,
                    Switch::new(SharedString::from(format!("page-{}", page.label())))
                        .checked(shown)
                        .on_click(cx.listener(move |this, on: &bool, window, cx| {
                            let on = *on;
                            this.update_config(window, cx, |c| {
                                if on {
                                    let mut pages: Vec<Page> = Page::ALL
                                        .iter()
                                        .copied()
                                        .filter(|p| c.layout.pages.contains(p) || *p == page)
                                        .collect();
                                    pages.dedup();
                                    c.layout.pages = pages;
                                } else {
                                    c.layout.pages.retain(|p| *p != page);
                                }
                            });
                        })),
                    i,
                    cx,
                )
            }),
    );

    let path = app.store.path().to_string_lossy().to_string();
    let themes_dir = crate::config::themes_dir();
    let theme = cx.theme();
    let footer = div()
        .v_flex()
        .gap_2()
        .mt_6()
        .px_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(
            div()
                .h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .font_family(theme.mono_font_family.clone())
                        .truncate()
                        .child(path),
                )
                .child(
                    Button::new("open-config")
                        .ghost()
                        .xsmall()
                        .icon(IconName::FolderOpen)
                        .label("Open folder")
                        .on_click(|_, _, cx| {
                            let dir = crate::config::dir();
                            crate::config::ensure_dir(&crate::config::themes_dir()).ok();
                            cx.open_with_system(&dir);
                        }),
                )
                .child(
                    Button::new("reset-config")
                        .ghost()
                        .xsmall()
                        .label("Reset")
                        .on_click(cx.listener(|this, _, window, cx| {
                            let pages = Config::default();
                            this.appearance.sync(&pages, window, cx);
                            this.update_config(window, cx, |c| *c = pages);
                        })),
                ),
        )
        .child(
            div()
                .font_family(theme.mono_font_family.clone())
                .truncate()
                .child(format!("{}/*.json", themes_dir.to_string_lossy())),
        );

    div()
        .v_flex()
        .child(kit::header("Appearance", cx).child(mode_switch))
        .child(themes)
        .child(kit::section("Accent", cx))
        .child(kit::card(cx).child(accent_row))
        .child(kit::section("Style", cx))
        .child(style)
        .child(kit::section("Wi-Fi list", cx))
        .child(wifi)
        .child(kit::section("Devices", cx))
        .child(devices)
        .child(kit::section("Behavior", cx))
        .child(behavior)
        .child(kit::section("Pages", cx))
        .child(pages)
        .child(footer)
        .into_any_element()
}
