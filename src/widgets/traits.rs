use crossterm::event::KeyCode;

use crate::app_state::AppState;

pub trait InputWidget {
    fn on_input(&mut self, key: KeyCode, state: &mut AppState);
}