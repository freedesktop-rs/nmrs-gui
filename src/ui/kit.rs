use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme, Icon, Sizable, StyledExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::NmApp;

pub fn header(title: &str, cx: &App) -> Div {
    let _ = cx;
    div().h_flex().items_center().gap_2().mb_5().child(
        div()
            .flex_1()
            .text_xl()
            .font_semibold()
            .child(title.to_string()),
    )
}

pub fn section(title: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .h_flex()
        .items_center()
        .gap_2()
        .mt_6()
        .mb_2()
        .px_1()
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(title.into())
}

pub fn card(cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .v_flex()
        .w_full()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(theme.border)
        .bg(theme.secondary)
        .overflow_hidden()
}

pub fn divided(index: usize, cx: &App) -> impl Fn(Div) -> Div {
    let border = cx.theme().border;
    move |s: Div| {
        if index > 0 {
            s.border_t_1().border_color(border)
        } else {
            s
        }
    }
}

pub const ROW_CONTEXT: &str = "NmrsRow";

pub fn focusable(
    el: Stateful<Div>,
    id: impl Into<ElementId>,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let primary = cx.theme().primary;
    let handle = window
        .use_keyed_state(id, cx, |_, cx| cx.focus_handle())
        .read(cx)
        .clone();
    el.key_context(ROW_CONTEXT)
        .track_focus(&handle.tab_index(0).tab_stop(true))
        .focus_visible(|s| s.bg(primary.opacity(0.18)))
}

pub fn row(
    id: impl Into<ElementId> + Clone,
    height: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let accent = cx.theme().accent;
    let row = div()
        .id(id.clone())
        .h_flex()
        .items_center()
        .gap_3()
        .min_h(height)
        .px_3p5()
        .hover(|s| s.bg(accent.opacity(0.45)));
    focusable(row, id, window, cx)
}

pub fn title_block(title: impl Into<SharedString>, subtitle: Option<String>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .v_flex()
        .flex_1()
        .min_w_0()
        .py_1p5()
        .child(div().text_sm().truncate().child(title.into()))
        .when_some(subtitle.filter(|s| !s.is_empty()), |s, sub| {
            s.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .truncate()
                    .child(sub),
            )
        })
}

pub fn tag(text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex_shrink_0()
        .px_1p5()
        .py_px()
        .rounded(px(4.))
        .bg(theme.accent)
        .text_color(theme.muted_foreground)
        .text_xs()
        .child(text.into())
}

pub fn accent_tag(text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex_shrink_0()
        .px_1p5()
        .py_px()
        .rounded(px(4.))
        .bg(theme.primary.opacity(0.14))
        .text_color(theme.primary)
        .text_xs()
        .font_medium()
        .child(text.into())
}

pub fn badge_icon(icon: IconName, active: bool, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex_shrink_0()
        .size_8()
        .rounded(theme.radius)
        .flex()
        .items_center()
        .justify_center()
        .when(active, |s| {
            s.bg(theme.primary.opacity(0.14)).text_color(theme.primary)
        })
        .when(!active, |s| {
            s.bg(theme.accent).text_color(theme.muted_foreground)
        })
        .child(Icon::new(icon).size_4())
}

pub fn signal_icon(strength: u8) -> IconName {
    match strength {
        0..=24 => IconName::WifiZero,
        25..=49 => IconName::WifiLow,
        50..=74 => IconName::WifiHigh,
        _ => IconName::Wifi,
    }
}

pub fn kv_grid(items: Vec<(&'static str, String)>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .grid()
        .grid_cols(2)
        .gap_x_6()
        .gap_y_3()
        .px_3p5()
        .py_3()
        .children(items.into_iter().map(|(label, value)| {
            div()
                .v_flex()
                .min_w_0()
                .gap_0p5()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(label),
                )
                .child(div().text_sm().truncate().child(value))
        }))
}

pub fn field(label: &'static str, input: &Entity<InputState>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .v_flex()
        .gap_1p5()
        .min_w_0()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(Input::new(input).small())
}

pub fn secret_field(label: &'static str, input: &Entity<InputState>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .v_flex()
        .gap_1p5()
        .min_w_0()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(Input::new(input).small().mask_toggle())
}

pub fn labeled(label: &'static str, child: impl IntoElement, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .v_flex()
        .gap_1p5()
        .min_w_0()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(child)
}

pub fn pair(a: impl IntoElement, b: impl IntoElement) -> Div {
    div().grid().grid_cols(2).gap_3().child(a).child(b)
}

pub fn form(cx: &App) -> Div {
    let _ = cx;
    div().v_flex().gap_3().px_3p5().py_3p5()
}

pub fn actions() -> Div {
    div().h_flex().items_center().justify_end().gap_2().pt_1()
}

pub fn segmented<T: Copy + PartialEq + 'static, L: Into<SharedString> + Clone>(
    id: &'static str,
    options: &[(T, L)],
    selected: T,
    cx: &mut Context<NmApp>,
    on_select: impl Fn(&mut NmApp, T, &mut Window, &mut Context<NmApp>) + Clone + 'static,
) -> Div {
    let theme = cx.theme();
    let (bg, radius, active_bg, border) =
        (theme.accent, theme.radius, theme.background, theme.border);
    let muted = theme.muted_foreground;
    let fg = theme.foreground;
    div()
        .h_flex()
        .flex_shrink_0()
        .p_0p5()
        .gap_0p5()
        .rounded(radius)
        .bg(bg)
        .children(options.iter().enumerate().map(|(i, (value, label))| {
            let value = *value;
            let active = value == selected;
            let on_select = on_select.clone();
            div()
                .id(SharedString::from(format!("{id}-{i}")))
                .px_2p5()
                .h_6()
                .flex()
                .items_center()
                .rounded(radius * 0.75)
                .text_xs()
                .cursor_pointer()
                .when(active, |s| {
                    s.bg(active_bg)
                        .text_color(fg)
                        .border_1()
                        .border_color(border)
                })
                .when(!active, |s| s.text_color(muted).hover(|s| s.text_color(fg)))
                .child(label.clone().into())
                .on_click(
                    cx.listener(move |this, _, window, cx| on_select(this, value, window, cx)),
                )
        }))
}

pub fn setting_row(
    title: &'static str,
    detail: Option<String>,
    control: impl IntoElement,
    index: usize,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    div()
        .h_flex()
        .items_center()
        .gap_4()
        .min_h_12()
        .px_3p5()
        .py_2()
        .map(divided(index, cx))
        .child(
            div()
                .v_flex()
                .flex_1()
                .min_w_0()
                .child(div().text_sm().child(title))
                .when_some(detail, |s, d| {
                    s.child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .truncate()
                            .child(d),
                    )
                }),
        )
        .child(control)
}

pub fn empty(icon: IconName, title: &'static str, action: Option<Button>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .v_flex()
        .items_center()
        .justify_center()
        .gap_3()
        .py_16()
        .text_color(theme.muted_foreground)
        .child(Icon::new(icon).size_6())
        .child(div().text_sm().child(title))
        .when_some(action, |s, a| s.child(a))
}

pub fn primary(id: impl Into<ElementId>, label: &'static str, loading: bool) -> Button {
    Button::new(id)
        .primary()
        .small()
        .label(label)
        .loading(loading)
}

pub fn plain(id: impl Into<ElementId>, label: &'static str) -> Button {
    Button::new(id).small().label(label)
}

pub fn danger(id: impl Into<ElementId>, label: &'static str, armed: bool) -> Button {
    let button = Button::new(id).small();
    if armed {
        button.danger().label("Confirm")
    } else {
        button.ghost().label(label)
    }
}
