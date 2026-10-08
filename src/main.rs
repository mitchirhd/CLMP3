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

    let mut app = App::new()?;

    let app_result = app.start(&mut terminal);

    ratatui::restore();
    app_result
}
