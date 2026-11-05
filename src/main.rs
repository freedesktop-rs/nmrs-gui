mod app;
mod assets;
mod backend;
mod compositor;
mod config;
mod ipc;
mod model;
mod theme;
mod ui;

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use clap::{ArgAction, Parser};
use futures::StreamExt;
use gpui_kit::*;

use crate::app::NmApp;
use crate::backend::Backend;
use crate::config::{LayoutMode, Page, Store};
use crate::ipc::Command;

#[derive(Parser, Debug)]
#[command(name = "nmrs-gui", disable_version_flag = true)]
struct Args {
    #[arg(short = 'V', long = "version", action = ArgAction::SetTrue)]
    version: bool,
    #[arg(long, value_enum)]
    page: Option<Page>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, value_enum)]
    layout: Option<LayoutMode>,
    #[arg(long, default_value = "nmrs")]
    app_id: String,
    #[arg(long)]
    demo: bool,
    #[arg(
        long,
        group = "command",
        help = "Show or hide the running window; with --page, switch to that page first"
    )]
    toggle: bool,
    #[arg(long, group = "command", help = "Show and raise the running window")]
    show: bool,
    #[arg(
        long,
        group = "command",
        help = "Hide the running window, keeping it ready"
    )]
    hide: bool,
    #[arg(long, group = "command", help = "Quit the running instance")]
    quit: bool,
}

impl Args {
    fn command(&self) -> Option<Command> {
        if self.toggle {
            Some(Command::Toggle(self.page))
        } else if self.show {
            Some(Command::Show(self.page))
        } else if self.hide {
            Some(Command::Hide)
        } else if self.quit {
            Some(Command::Quit)
        } else {
            None
        }
    }
}

struct Launcher {
    backend: Backend,
    config_path: Option<PathBuf>,
    layout: Option<LayoutMode>,
    app_id: String,
    hiding: Cell<bool>,
    app: RefCell<Option<WeakEntity<NmApp>>>,
}

impl Launcher {
    fn window(&self, cx: &App) -> Option<AnyWindowHandle> {
        cx.windows().into_iter().next()
    }

    fn handle(&self, command: Command, cx: &mut App) -> String {
        match command {
            Command::Show(page) => match self.window(cx) {
                Some(handle) => self.raise(handle, page, cx),
                None => self.open(page, cx),
            },
            Command::Hide => match self.window(cx) {
                Some(handle) => self.hide(handle, cx),
                None => "ok".into(),
            },
            Command::Toggle(page) => match self.window(cx) {
                Some(handle) if page.is_some_and(|p| self.page(cx) != Some(p)) => {
                    self.raise(handle, page, cx)
                }
                Some(handle) => self.hide(handle, cx),
                None => self.open(page, cx),
            },
            Command::Quit => {
                cx.quit();
                "ok".into()
            }
        }
    }

    fn page(&self, cx: &App) -> Option<Page> {
        let app = self.app.borrow().as_ref().and_then(|app| app.upgrade())?;
        Some(app.read(cx).page)
    }

    fn raise(&self, handle: AnyWindowHandle, page: Option<Page>, cx: &mut App) -> String {
        let app = self.app.borrow().as_ref().and_then(|app| app.upgrade());
        let result = handle.update(cx, |_, window, cx| {
            if let (Some(page), Some(app)) = (page, app) {
                app.update(cx, |app, cx| app.go(page, window, cx));
            }
            window.activate_window();
        });
        match result {
            Ok(()) => "ok".into(),
            Err(err) => format!("error: {err}"),
        }
    }

    fn hide(&self, handle: AnyWindowHandle, cx: &mut App) -> String {
        self.hiding.set(true);
        match handle.update(cx, |_, window, _| window.remove_window()) {
            Ok(()) => "ok".into(),
            Err(err) => {
                self.hiding.set(false);
                format!("error: {err}")
            }
        }
    }

    fn open(&self, page: Option<Page>, cx: &mut App) -> String {
        let mut store = Store::new(self.config_path.clone());
        let (mut config, config_error): (config::Config, _) = match store.load() {
            Ok(config) => (config, None),
            Err(err) => (Default::default(), Some(err)),
        };
        if let Some(layout) = self.layout {
            config.layout.mode = layout;
        }
        let backend = self.backend.clone();
        let app_id = self.app_id.clone();

        let size = size(px(config.layout.width), px(config.layout.height));
        let options = WindowOptions {
            app_id: Some(app_id.clone()),
            titlebar: Some(TitlebarOptions {
                title: Some("nmrs".into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::centered(size, cx)),
            window_min_size: Some(gpui_kit::size(px(360.), px(420.))),
            window_background: if config.appearance.opacity < 1.0 {
                WindowBackgroundAppearance::Transparent
            } else {
                WindowBackgroundAppearance::Opaque
            },
            ..Default::default()
        };

        let opened = gpui_kit::open_window(options, cx, move |window, cx| {
            theme::apply(&config.appearance, Some(window), cx);
            cx.new(|cx| {
                let app = NmApp::new(backend, store, config, page, app_id, window, cx);
                if let Some(err) = config_error {
                    use gpui_kit::component::WindowExt;
                    window.push_notification(
                        gpui_kit::component::notification::Notification::error(err)
                            .title("Config failed to load"),
                        cx,
                    );
                }
                app
            })
        });
        match opened {
            Ok((_, app)) => {
                *self.app.borrow_mut() = Some(app.downgrade());
                "ok".into()
            }
            Err(err) => format!("error: failed to open window: {err}"),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.version {
        println!(
            "nmrs-gui {} ({})",
            env!("CARGO_PKG_VERSION"),
            env!("GIT_HASH")
        );
        return Ok(());
    }

    let command = args.command();
    if let Some(mut stream) = ipc::connect(&args.app_id) {
        let command = command.unwrap_or(Command::Show(args.page));
        let reply = ipc::send(&mut stream, &command)?;
        if reply != "ok" {
            eprintln!("{reply}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if matches!(command, Some(Command::Hide | Command::Quit)) {
        eprintln!("nmrs-gui is not running");
        std::process::exit(1);
    }

    let socket = ipc::socket_path(&args.app_id);
    let requests = ipc::serve(ipc::bind(&args.app_id)?);
    let backend = Backend::start(args.demo)?;
    let launcher = Rc::new(Launcher {
        backend,
        config_path: args.config,
        layout: args.layout,
        app_id: args.app_id,
        hiding: Cell::new(false),
        app: RefCell::new(None),
    });
    let page = args.page;

    gpui_kit::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.set_quit_mode(QuitMode::Explicit);
            app::bind_keys(cx);
            for err in theme::load(cx) {
                log::warn!("{err}");
            }

            let reply = launcher.open(page, cx);
            if reply != "ok" {
                eprintln!("{reply}");
                cx.quit();
                return;
            }

            let closed = launcher.clone();
            cx.on_window_closed(move |cx, _| {
                if !closed.hiding.replace(false) {
                    cx.quit();
                }
            })
            .detach();

            let served = launcher.clone();
            cx.spawn(async move |cx| {
                let mut requests = requests;
                while let Some((command, reply)) = requests.next().await {
                    let result = cx.update(|cx| served.handle(command, cx));
                    let _ = reply.send(result);
                }
            })
            .detach();
        });
    std::fs::remove_file(socket).ok();
    Ok(())
}
