use crossterm::event::{KeyCode, KeyModifiers};

use crate::app_state::AppState;

pub trait InputWidget {
    fn on_input(
        &mut self,
        modifier: KeyModifiers,
        key: KeyCode,
        state: &mut AppState,
    ) -> anyhow::Result<()>;
}
