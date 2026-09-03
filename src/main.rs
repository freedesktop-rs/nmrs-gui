mod model;

use clap::{ArgAction, Parser};
use gpui_kit::*;

#[derive(Parser, Debug)]
#[command(name = "nmrs-gui", disable_version_flag = true)]
struct Args {
    #[arg(short = 'V', long = "version", action = ArgAction::SetTrue)]
    version: bool,
}

struct Shell;

impl Render for Shell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.version {
        println!("nmrs-gui {} ({})", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
        return Ok(());
    }

    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        let options = WindowOptions {
            app_id: Some("nmrs".into()),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| Shell)).ok();
    });
    Ok(())
}
