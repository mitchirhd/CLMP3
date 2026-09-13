use std::io;
mod widgets;
mod app;
mod events;
use app::App;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App::new();

    let app_result = app.start(&mut terminal);

    ratatui::restore();
    app_result
}
