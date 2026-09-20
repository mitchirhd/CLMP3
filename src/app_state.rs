use ratatui::widgets::ListState;

pub struct AppState {
    pub queue_state: ListState,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            queue_state: ListState::default(),
        }
    }
}
