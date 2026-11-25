use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use futures::channel::{mpsc, oneshot};
use futures::future::{self, Either};
use futures::stream::Fuse;
use futures::{FutureExt, StreamExt};
use nmrs::ConnectionError;
use nmrs::agent::{
    CancelReason, SecretAgent, SecretAgentCapabilities, SecretAgentFlags, SecretAgentHandle,
    SecretRequest, SecretSetting,
};
use nmrs::raw::zbus;
use nmrs::raw::zbus::fdo::{NameOwnerChanged, NameOwnerChangedStream};
use nmrs::raw::zvariant::{OwnedValue, Str};

use super::SecretEvent;
use crate::model::{SecretField, SecretKind, SecretPrompt};

const NM_BUS: &str = "org.freedesktop.NetworkManager";
const IDENTIFIER: &str = "org.nmrs.gui";
const WIFI_KEYS: [&str; 6] = [
    "psk",
    "wep-key0",
    "wep-key1",
    "wep-key2",
    "wep-key3",
    "leap-password",
];
const VPN_MESSAGE: &str = "x-vpn-message:";
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

type Owners = Fuse<NameOwnerChangedStream>;

enum Event {
    Request(Option<Box<SecretRequest>>),
    Cancel(Option<CancelReason>),
    Owner(Option<NameOwnerChanged>),
}

pub async fn run(tx: mpsc::UnboundedSender<SecretEvent>) -> Result<()> {
    let bus = zbus::Connection::system()
        .await
        .context("connecting to the system bus")?;
    let dbus = zbus::fdo::DBusProxy::new(&bus).await?;
    let mut owners = dbus
        .receive_name_owner_changed_with_args(&[(0, NM_BUS)])
        .await?
        .fuse();

    let Some((mut handle, mut requests)) = retry(&mut owners, register).await? else {
        log::info!("another nmrs-gui instance is the secret agent; running without one");
        return Ok(());
    };
    loop {
        let event = futures::select_biased! {
            req = requests.next() => Event::Request(req.map(Box::new)),
            cancel = handle.cancellations().next() => Event::Cancel(cancel),
            owner = owners.next() => Event::Owner(owner),
        };
        match event {
            Event::Request(Some(req)) => dispatch(*req, &tx),
            Event::Request(None) => bail!("NetworkManager request stream closed"),
            Event::Cancel(Some(reason)) => {
                let _ = tx.unbounded_send(SecretEvent::Cancelled {
                    path: reason.connection_path.to_string(),
                    setting: reason.setting_name,
                });
            }
            Event::Cancel(None) => {}
            Event::Owner(owner) => {
                if appeared(owner)? {
                    retry(&mut owners, || reregister(&handle)).await?;
                }
            }
        }
        while handle.store_events().try_recv().is_ok() {}
    }
}

async fn register() -> nmrs::Result<Option<(SecretAgentHandle, mpsc::Receiver<SecretRequest>)>> {
    let result = SecretAgent::builder()
        .with_identifier(IDENTIFIER)
        .with_capabilities(SecretAgentCapabilities::VPN_HINTS)
        .register()
        .await;
    match result {
        Err(ConnectionError::AgentAlreadyRegistered) => Ok(None),
        result => result.map(Some),
    }
}

async fn reregister(handle: &SecretAgentHandle) -> nmrs::Result<()> {
    match handle.reregister().await {
        Err(ConnectionError::AgentAlreadyRegistered) => Ok(()),
        result => result,
    }
}

async fn retry<T, F: Future<Output = nmrs::Result<T>>>(
    owners: &mut Owners,
    mut op: impl FnMut() -> F,
) -> Result<T> {
    let mut delay = Duration::from_secs(1);
    loop {
        match op().await {
            Ok(value) => return Ok(value),
            Err(err) => log::warn!("secret agent registration failed, retrying: {err}"),
        }
        let timer = std::pin::pin!(tokio::time::sleep(delay));
        if let Either::Left((owner, _)) = future::select(owners.next(), timer).await {
            appeared(owner)?;
        }
        delay = (delay * 2).min(MAX_BACKOFF);
    }
}

fn appeared(signal: Option<NameOwnerChanged>) -> Result<bool> {
    let Some(signal) = signal else {
        bail!("NameOwnerChanged stream closed");
    };
    Ok(signal.args()?.new_owner.is_some())
}

fn dispatch(req: SecretRequest, tx: &mpsc::UnboundedSender<SecretEvent>) {
    let responder = req.responder;
    let prompt = req
        .flags
        .contains(SecretAgentFlags::ALLOW_INTERACTION)
        .then(|| prompt(&req.setting, &req.hints, req.existing_secrets))
        .flatten();
    let Some((setting, kind, fields)) = prompt else {
        log::debug!(
            "no secrets for {} ({:?}, {:?})",
            req.connection_id,
            req.setting,
            req.flags
        );
        let _ = responder.no_secrets().now_or_never();
        return;
    };

    let path = req.connection_path.to_string();
    let (reply_tx, reply_rx) = oneshot::channel();
    let prompt = SecretPrompt {
        uuid: req.connection_uuid,
        connection: req.connection_id,
        path: path.clone(),
        setting: setting.into(),
        kind,
        fields,
    };
    if tx
        .unbounded_send(SecretEvent::Request(prompt, reply_tx))
        .is_err()
    {
        let _ = responder.no_secrets().now_or_never();
        return;
    }
    let tx = tx.clone();
    tokio::spawn(async move {
        let timer = std::pin::pin!(tokio::time::sleep(RESPONSE_TIMEOUT));
        let result = match future::select(reply_rx, timer).await {
            Either::Left((Ok(secrets), _)) if setting == "vpn" => {
                responder.vpn_secrets(secrets).await
            }
            Either::Left((Ok(secrets), _)) => {
                let secrets = secrets
                    .into_iter()
                    .map(|(k, v)| (k, OwnedValue::from(Str::from(v))))
                    .collect();
                responder.raw(setting, secrets).await
            }
            Either::Left((Err(_), _)) => responder.cancel().await,
            Either::Right(_) => {
                let setting = setting.into();
                let _ = tx.unbounded_send(SecretEvent::Cancelled { path, setting });
                responder.no_secrets().await
            }
        };
        if let Err(err) = result {
            log::debug!("secret reply not delivered: {err}");
        }
    });
}

fn prompt(
    setting: &SecretSetting,
    hints: &[String],
    existing: HashMap<String, String>,
) -> Option<(&'static str, SecretKind, Vec<SecretField>)> {
    let hinted: Vec<String> = hints
        .iter()
        .filter(|h| !h.starts_with("x-"))
        .cloned()
        .collect();
    let password = || vec!["password".to_string()];
    let (setting, kind, keys) = match setting {
        SecretSetting::WifiPsk { ssid } => {
            let key = hints
                .iter()
                .find(|h| WIFI_KEYS.contains(&h.as_str()))
                .map_or("psk", String::as_str);
            let kind = SecretKind::Wifi { ssid: ssid.clone() };
            ("802-11-wireless-security", kind, vec![key.into()])
        }
        SecretSetting::WifiEap { identity, .. } => {
            let kind = SecretKind::Eap {
                identity: identity.clone(),
            };
            let keys = if hinted.is_empty() {
                password()
            } else {
                hinted
            };
            ("802-1x", kind, keys)
        }
        SecretSetting::Vpn { .. } => {
            let message = hints
                .iter()
                .find_map(|h| h.strip_prefix(VPN_MESSAGE))
                .map(String::from);
            let keys = if !hinted.is_empty() {
                hinted
            } else if !existing.is_empty() {
                let mut keys: Vec<String> = existing.keys().cloned().collect();
                keys.sort();
                keys
            } else {
                password()
            };
            ("vpn", SecretKind::Vpn { message }, keys)
        }
        _ => return None,
    };
    let fields = keys
        .into_iter()
        .map(|key| SecretField {
            masked: !key.to_ascii_lowercase().contains("user"),
            value: existing.get(&key).cloned(),
            key,
        })
        .collect();
    Some((setting, kind, fields))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints(h: &[&str]) -> Vec<String> {
        h.iter().map(|s| s.to_string()).collect()
    }

    fn keys(setting: &SecretSetting, h: &[&str], existing: &[(&str, &str)]) -> Vec<String> {
        let existing = existing
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let (_, _, fields) = prompt(setting, &hints(h), existing).unwrap();
        fields.into_iter().map(|f| f.key).collect()
    }

    #[test]
    fn wifi_uses_the_hinted_key() {
        let wifi = SecretSetting::WifiPsk { ssid: "x".into() };
        assert_eq!(keys(&wifi, &["wep-key0"], &[]), ["wep-key0"]);
        assert_eq!(keys(&wifi, &[], &[]), ["psk"]);
    }

    #[test]
    fn eap_asks_for_the_hinted_secret() {
        let eap = SecretSetting::WifiEap {
            identity: None,
            method: Some("tls".into()),
        };
        assert_eq!(
            keys(&eap, &["private-key-password"], &[]),
            ["private-key-password"]
        );
        assert_eq!(keys(&eap, &[], &[]), ["password"]);
    }

    #[test]
    fn vpn_splits_message_from_keys() {
        let vpn = SecretSetting::Vpn {
            service_type: "org.freedesktop.NetworkManager.openvpn".into(),
            user_name: None,
        };
        let Some((_, SecretKind::Vpn { message }, _)) = prompt(
            &vpn,
            &hints(&["x-vpn-message:Enter your token"]),
            HashMap::new(),
        ) else {
            panic!()
        };
        assert_eq!(message.as_deref(), Some("Enter your token"));
        assert_eq!(
            keys(&vpn, &["x-vpn-message:hi", "challenge-response"], &[]),
            ["challenge-response"]
        );
        assert_eq!(
            keys(&vpn, &[], &[("username", "a"), ("password", "b")]),
            ["password", "username"]
        );
        assert_eq!(keys(&vpn, &[], &[]), ["password"]);
    }

    #[test]
    fn usernames_are_prefilled_and_unmasked() {
        let vpn = SecretSetting::Vpn {
            service_type: String::new(),
            user_name: None,
        };
        let existing = HashMap::from([("username".to_string(), "akrm".to_string())]);
        let (_, _, fields) = prompt(&vpn, &hints(&["username"]), existing).unwrap();
        assert!(!fields[0].masked);
        assert_eq!(fields[0].value.as_deref(), Some("akrm"));
    }

    #[test]
    fn unsupported_settings_get_no_prompt() {
        assert!(prompt(&SecretSetting::Gsm, &[], HashMap::new()).is_none());
    }
}
