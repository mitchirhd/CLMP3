use std::io;
mod widgets;
mod app;
mod enums;
use app::App;
mod audio;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App::new();

    let app_result = app.start(&mut terminal);

    ratatui::restore();
    app_result
}
