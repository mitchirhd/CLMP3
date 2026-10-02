use crossterm::event::KeyEvent;

pub enum Clmp3Event {
    KeyInput(KeyEvent),
    // MouseInput(MouseEvent),
    Redraw,
    SongEnded,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Clmp3Widget {
    Player,
    Display,
    Load,
    Queue,
}

impl Clmp3Widget {
    pub fn area(&self) -> WidgetArea {
        match self {
            Clmp3Widget::Player => WidgetArea::Player,
            Clmp3Widget::Display => WidgetArea::Display,
            Clmp3Widget::Load => WidgetArea::Display,
            Clmp3Widget::Queue => WidgetArea::Queue,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WidgetArea {
    Player,
    Display,
    Queue,
}
