use std::collections::VecDeque;

use futures::channel::oneshot;
use gpui_kit::assets::IconName;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme, StyledExt, WindowExt};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::app::{NmApp, input, set};
use crate::model::{Action, SecretAnswer, SecretField, SecretKind, SecretPrompt};
use crate::ui::kit;

#[derive(Default)]
pub struct SecretsUi {
    queue: VecDeque<Pending>,
    cancelled: Vec<(u64, SecretPrompt)>,
    seq: u64,
}

impl SecretsUi {
    pub fn mark(&self) -> u64 {
        self.seq
    }

    pub fn cancelled_since(&self, mark: u64, action: &Action) -> bool {
        self.cancelled
            .iter()
            .any(|(seq, prompt)| *seq > mark && targets(action, prompt))
    }

    pub fn settle(&mut self) {
        self.cancelled.clear();
    }

    fn cancel(&mut self, prompt: SecretPrompt) {
        self.seq += 1;
        self.cancelled.push((self.seq, prompt));
    }
}

fn targets(action: &Action, prompt: &SecretPrompt) -> bool {
    match action {
        Action::Activate(uuid) | Action::Reconnect(uuid, _) => *uuid == prompt.uuid,
        Action::Connect { ssid, .. } => {
            prompt.connection == *ssid
                || matches!(&prompt.kind, SecretKind::Wifi { ssid: s } if s == ssid)
        }
        _ => false,
    }
}

struct Pending {
    prompt: SecretPrompt,
    reply: oneshot::Sender<SecretAnswer>,
    inputs: Vec<Entity<InputState>>,
    _subscriptions: Vec<Subscription>,
}

impl Pending {
    fn values(&self, cx: &App) -> impl Iterator<Item = (&SecretField, String)> {
        self.prompt
            .fields
            .iter()
            .zip(&self.inputs)
            .map(|(f, input)| {
                let value = input.read(cx).value();
                let value: &str = if f.masked { &value } else { value.trim() };
                (f, value.to_string())
            })
    }
}

pub fn push(
    app: &mut NmApp,
    prompt: SecretPrompt,
    reply: oneshot::Sender<SecretAnswer>,
    window: &mut Window,
    cx: &mut Context<NmApp>,
) {
    let inputs: Vec<_> = prompt
        .fields
        .iter()
        .map(|f| {
            let input = input("", f.masked, window, cx);
            if let Some(text) = &f.value {
                set(&input, text, window, cx);
            }
            input
        })
        .collect();
    let subscriptions = inputs
        .iter()
        .map(|input| {
            cx.subscribe_in(input, window, |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    submit(this, window, cx);
                }
            })
        })
        .collect();
    app.secrets.queue.push_back(Pending {
        prompt,
        reply,
        inputs,
        _subscriptions: subscriptions,
    });
    if app.secrets.queue.len() == 1 {
        focus(app, window, cx);
    }
    cx.notify();
}

pub fn cancelled(
    app: &mut NmApp,
    path: &str,
    setting: &str,
    window: &mut Window,
    cx: &mut Context<NmApp>,
) {
    let matches = |p: &Pending| p.prompt.path == path && p.prompt.setting == setting;
    let queue = &mut app.secrets.queue;
    let Some(connection) = queue
        .iter()
        .find(|p| matches(p))
        .map(|p| p.prompt.connection.clone())
    else {
        return;
    };
    let front = queue.front().is_some_and(matches);
    queue.retain(|p| !matches(p));
    window.push_notification(
        Notification::info(format!(
            "The request for “{connection}” expired or was withdrawn"
        ))
        .title("Credentials no longer needed"),
        cx,
    );
    if front {
        focus(app, window, cx);
    }
    cx.notify();
}

pub fn dismiss(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) -> bool {
    let Some(pending) = app.secrets.queue.pop_front() else {
        return false;
    };
    app.secrets.cancel(pending.prompt);
    focus(app, window, cx);
    cx.notify();
    true
}

fn submit(app: &mut NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let Some(pending) = app.secrets.queue.front() else {
        return;
    };
    let empty = pending.values(cx).position(|(_, v)| v.is_empty());
    if let Some(index) = empty {
        let input = pending.inputs[index].clone();
        input.update(cx, |state, cx| state.focus(window, cx));
        return;
    }
    let answer: SecretAnswer = pending
        .values(cx)
        .map(|(f, v)| (f.key.clone(), v))
        .collect();
    if let Some(pending) = app.secrets.queue.pop_front() {
        let _ = pending.reply.send(answer);
    }
    focus(app, window, cx);
    cx.notify();
}

fn focus(app: &NmApp, window: &mut Window, cx: &mut Context<NmApp>) {
    let Some(pending) = app.secrets.queue.front() else {
        app.refocus(window, cx);
        return;
    };
    let index = pending
        .values(cx)
        .position(|(_, v)| v.is_empty())
        .unwrap_or(0);
    if let Some(input) = pending.inputs.get(index).cloned() {
        input.update(cx, |state, cx| state.focus(window, cx));
    }
}

fn label(key: &str) -> String {
    match key {
        "psk" | "leap-password" => return "Password".into(),
        k if k.starts_with("wep-key") => return "WEP key".into(),
        _ => {}
    }
    let text = key.replace(['-', '_'], " ");
    let mut chars = text.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

pub fn render(app: &NmApp, cx: &mut Context<NmApp>) -> Option<AnyElement> {
    let pending = app.secrets.queue.front()?;
    let prompt = &pending.prompt;
    let (icon, title, subtitle) = match &prompt.kind {
        SecretKind::Wifi { ssid, .. } => (
            IconName::Lock,
            "Wi-Fi password required",
            format!("Enter the password for “{ssid}”"),
        ),
        SecretKind::Eap { identity } => (
            IconName::Lock,
            "Authentication required",
            match identity {
                Some(id) => format!("Sign in to “{}” as {id}", prompt.connection),
                None => format!("Sign in to “{}”", prompt.connection),
            },
        ),
        SecretKind::Vpn { message, .. } => (
            IconName::Shield,
            "VPN credentials required",
            message
                .clone()
                .unwrap_or_else(|| format!("“{}” needs credentials to connect", prompt.connection)),
        ),
    };
    let queued = app.secrets.queue.len() - 1;
    let theme = cx.theme();

    let fields = prompt.fields.iter().zip(&pending.inputs).map(|(f, input)| {
        if f.masked {
            kit::secret_field(label(&f.key), input, cx)
        } else {
            kit::field(label(&f.key), input, cx)
        }
    });

    Some(
        kit::card(cx)
            .border_color(theme.primary.opacity(0.5))
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .px_3p5()
                    .pt_3p5()
                    .child(kit::badge_icon(icon, true, cx))
                    .child(kit::title_block(title, Some(subtitle), cx))
                    .when(queued > 0, |s| {
                        s.child(kit::tag(format!("+{queued} more"), cx))
                    }),
            )
            .child(kit::form(cx).children(fields))
            .child(
                kit::actions()
                    .px_3p5()
                    .pb_3p5()
                    .child(kit::plain("secret-cancel", "Cancel").on_click(cx.listener(
                        |this, _, window, cx| {
                            dismiss(this, window, cx);
                        },
                    )))
                    .child(
                        kit::primary("secret-connect", "Connect", false)
                            .on_click(cx.listener(|this, _, window, cx| submit(this, window, cx))),
                    ),
            )
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::{SecretsUi, label};
    use crate::model::{Action, Credentials, SecretKind, SecretPrompt, Security};

    fn prompt(uuid: &str, connection: &str, kind: SecretKind) -> SecretPrompt {
        SecretPrompt {
            uuid: uuid.into(),
            connection: connection.into(),
            path: String::new(),
            setting: String::new(),
            kind,
            fields: Vec::new(),
        }
    }

    #[test]
    fn cancels_only_quiet_their_own_connection() {
        let mut ui = SecretsUi::default();
        let before = ui.mark();
        ui.cancel(prompt(
            "u1",
            "Home",
            SecretKind::Wifi {
                ssid: "Home".into(),
            },
        ));
        let after = ui.mark();

        assert!(ui.cancelled_since(before, &Action::Activate("u1".into())));
        assert!(ui.cancelled_since(before, &Action::Reconnect("u1".into(), "wlan0".into())));
        assert!(!ui.cancelled_since(before, &Action::Activate("u2".into())));
        assert!(!ui.cancelled_since(after, &Action::Activate("u1".into())));

        ui.settle();
        assert!(!ui.cancelled_since(before, &Action::Activate("u1".into())));
    }

    #[test]
    fn wifi_connects_match_by_ssid() {
        let mut ui = SecretsUi::default();
        ui.cancel(prompt(
            "u1",
            "Home 1",
            SecretKind::Wifi {
                ssid: "Home".into(),
            },
        ));
        ui.cancel(prompt("u2", "Office", SecretKind::Eap { identity: None }));
        let connect = |ssid: &str| Action::Connect {
            ssid: ssid.into(),
            interface: None,
            bssid: None,
            security: Security::default(),
            credentials: Credentials::None,
            hidden: false,
        };
        assert!(ui.cancelled_since(0, &connect("Home")));
        assert!(ui.cancelled_since(0, &connect("Office")));
        assert!(!ui.cancelled_since(0, &connect("Cafe")));
    }

    #[test]
    fn labels_read_naturally() {
        assert_eq!(label("psk"), "Password");
        assert_eq!(label("wep-key0"), "WEP key");
        assert_eq!(label("challenge-response"), "Challenge response");
        assert_eq!(label("Xauth password"), "Xauth password");
    }
}
