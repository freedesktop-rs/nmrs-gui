mod app;
mod assets;
mod backend;
mod config;
mod model;
mod theme;
mod ui;

use std::fs::File;
use std::path::PathBuf;

use clap::{ArgAction, Parser};
use gpui_kit::*;

use crate::app::NmApp;
use crate::backend::Backend;
use crate::config::{LayoutMode, Page, Store};

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
}

fn lock(app_id: &str) -> Option<File> {
    let dir = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
    let file = File::create(dir.join(format!("{app_id}.lock"))).ok()?;
    file.try_lock().ok()?;
    Some(file)
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

    let Some(_lock) = lock(&args.app_id) else {
        eprintln!("nmrs-gui is already running");
        std::process::exit(1);
    };

    let backend = Backend::start(args.demo)?;
    let mut store = Store::new(args.config);
    let (mut config, config_error): (config::Config, _) = match store.load() {
        Ok(config) => (config, None),
        Err(err) => (Default::default(), Some(err)),
    };
    if let Some(layout) = args.layout {
        config.layout.mode = layout;
    }
    let app_id = args.app_id;
    let page = args.page;

    gpui_kit::application()
        .with_assets(assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            app::bind_keys(cx);
            for err in theme::load(cx) {
                log::warn!("{err}");
            }

            let size = size(px(config.layout.width), px(config.layout.height));
            let options = WindowOptions {
                app_id: Some(app_id),
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
                    let app = NmApp::new(backend, store, config, page, window, cx);
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
            if let Err(err) = opened {
                eprintln!("failed to open window: {err}");
                cx.quit();
            }
            cx.on_window_closed(|cx, _| cx.quit()).detach();
        });
    Ok(())
}
