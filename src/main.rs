use crate::online_helper::OnlineHelper;
use app::App;

mod app;
mod app_state;
mod audio;
mod enums;
mod local_file_helper;
mod metadata;
mod metadata_loader;
mod online_helper;
mod source_builder;
mod source_queue;
mod widgets;

use clap::Parser;

#[derive(Parser)]
struct Args {
    /// Browser to take cookies from, e.g. firefox, chrome
    #[arg(long)]
    browser_for_cookies: Option<String>,

    /// Path to a file containing cookies
    #[arg(long)]
    cookies_path: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(browser) = args.browser_for_cookies {
        OnlineHelper::set_cookie_browser(browser);
    }
    if let Some(cookies_path) = args.cookies_path {
        OnlineHelper::set_cookie_path(cookies_path);
    }

    let mut terminal = ratatui::init();

    let app_result = App::new().and_then(|mut app| app.start(&mut terminal));

    ratatui::restore();
    app_result
}
