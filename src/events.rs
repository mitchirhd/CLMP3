use crossterm::event::{KeyEvent, MouseEvent};

pub enum CLMP3Event{
    KeyInput(KeyEvent),
    MouseInput(MouseEvent),
    Resize,
}