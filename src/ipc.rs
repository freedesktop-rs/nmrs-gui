use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use clap::ValueEnum;
use futures::channel::{mpsc, oneshot};

use crate::config::Page;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Show(Option<Page>),
    Hide,
    Toggle(Option<Page>),
    Quit,
}

impl Command {
    pub fn parse(line: &str) -> Option<Self> {
        let mut words = line.split_whitespace();
        let page = |w: Option<&str>| w.and_then(|p| Page::from_str(p, true).ok());
        match words.next()? {
            "show" => Some(Self::Show(page(words.next()))),
            "hide" => Some(Self::Hide),
            "toggle" => Some(Self::Toggle(page(words.next()))),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }

    fn encode(&self) -> String {
        let page = |p: &Option<Page>| {
            p.map(|p| format!(" {}", p.to_possible_value().unwrap().get_name()))
                .unwrap_or_default()
        };
        match self {
            Self::Show(p) => format!("show{}", page(p)),
            Self::Hide => "hide".into(),
            Self::Toggle(p) => format!("toggle{}", page(p)),
            Self::Quit => "quit".into(),
        }
    }
}

pub type Request = (Command, oneshot::Sender<String>);

pub fn socket_path(app_id: &str) -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("nmrs")
        .join(format!("{app_id}.sock"))
}

pub fn connect(app_id: &str) -> Option<UnixStream> {
    UnixStream::connect(socket_path(app_id)).ok()
}

pub fn send(stream: &mut UnixStream, command: &Command) -> std::io::Result<String> {
    writeln!(stream, "{}", command.encode())?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim().to_string())
}

pub fn bind(app_id: &str) -> std::io::Result<UnixListener> {
    let path = socket_path(app_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match UnixListener::bind(&path) {
        Ok(listener) => Ok(listener),
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            std::fs::remove_file(&path)?;
            UnixListener::bind(&path)
        }
        Err(err) => Err(err),
    }
}

pub fn serve(listener: UnixListener) -> mpsc::UnboundedReceiver<Request> {
    let (tx, rx) = mpsc::unbounded();
    std::thread::Builder::new()
        .name("nmrs-ipc".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let tx = tx.clone();
                std::thread::spawn(move || handle(stream, tx));
            }
        })
        .expect("spawn ipc thread");
    rx
}

fn handle(mut stream: UnixStream, tx: mpsc::UnboundedSender<Request>) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let reply = match Command::parse(&line) {
        Some(command) => {
            let (reply_tx, reply_rx) = oneshot::channel();
            if tx.unbounded_send((command, reply_tx)).is_ok() {
                futures::executor::block_on(reply_rx).unwrap_or_else(|_| "error: app gone".into())
            } else {
                "error: app gone".into()
            }
        }
        None => format!("error: unknown command {}", line.trim()),
    };
    let _ = writeln!(stream, "{reply}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn commands_round_trip() {
        for command in [
            Command::Show(None),
            Command::Show(Some(Page::Vpn)),
            Command::Hide,
            Command::Toggle(None),
            Command::Toggle(Some(Page::Profiles)),
            Command::Quit,
        ] {
            assert_eq!(Command::parse(&command.encode()), Some(command));
        }
    }

    #[::core::prelude::v1::test]
    fn unknown_input_is_rejected() {
        assert_eq!(Command::parse(""), None);
        assert_eq!(Command::parse("dance"), None);
        assert_eq!(Command::parse("show nowhere"), Some(Command::Show(None)));
    }

    #[::core::prelude::v1::test]
    fn second_bind_replaces_a_stale_socket() {
        let dir = std::env::temp_dir().join(format!("nmrs-ipc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stale.sock");
        drop(UnixListener::bind(&path).unwrap());
        assert!(path.exists());
        assert!(UnixStream::connect(&path).is_err());
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(_) => {
                std::fs::remove_file(&path).unwrap();
                UnixListener::bind(&path).unwrap()
            }
        };
        let rx = serve(listener);
        let mut client = UnixStream::connect(&path).unwrap();
        std::thread::spawn(move || {
            let (command, reply) = futures::executor::block_on(futures::StreamExt::into_future(rx))
                .0
                .unwrap();
            assert_eq!(command, Command::Toggle(Some(Page::Wifi)));
            reply.send("ok".into()).unwrap();
        });
        assert_eq!(
            send(&mut client, &Command::Toggle(Some(Page::Wifi))).unwrap(),
            "ok"
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
