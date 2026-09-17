use crossterm::event::{KeyEvent, MouseEvent};

pub enum CLMP3Event {
    KeyInput(KeyEvent),
    MouseInput(MouseEvent),
    Resize,
}

#[derive(PartialEq, Eq)]
pub enum CLMP3Widgets {
    Player,
    Display,
    Load,
    Queue,
}
