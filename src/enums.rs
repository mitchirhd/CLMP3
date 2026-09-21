use crossterm::event::KeyEvent;

pub enum CLMP3Event {
    KeyInput(KeyEvent),
    // MouseInput(MouseEvent),
    Redraw,
    SongEnded,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum CLMP3Widget {
    Player,
    Display,
    Load,
    Queue,
}

impl CLMP3Widget {
    pub fn get_area(&self) -> WidgetArea {
        match self {
            CLMP3Widget::Player => WidgetArea::Player,
            CLMP3Widget::Display => WidgetArea::Display,
            CLMP3Widget::Load => WidgetArea::Display,
            CLMP3Widget::Queue => WidgetArea::Queue,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WidgetArea {
    Player,
    Display,
    Queue,
}
