mod app;
mod enums;
mod widgets;
use app::App;
mod app_state;
mod audio;
mod metadata;
mod source_builder;
mod source_queue;

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();

    let app_result = App::new().and_then(|mut app| app.start(&mut terminal));

    ratatui::restore();
    app_result
}
