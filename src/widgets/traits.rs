use crossterm::event::KeyCode;

pub trait InputWidget{
    fn on_input(&mut self, key: KeyCode);
}