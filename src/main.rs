use std::io;
mod app;
mod enums;
mod widgets;
use app::App;
mod audio;
mod source_builder;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App::new();

    let app_result = app.start(&mut terminal);

    ratatui::restore();
    app_result
}
