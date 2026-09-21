use std::time::SystemTime;

use ratatui::widgets::ListState;
use tui_slider::SliderState;

use crate::enums::{CLMP3Widget, WidgetArea};

#[derive(Clone)]
pub struct AppState {
    selected_widget: CLMP3Widget,
    last_display_widget: CLMP3Widget,
    volume_changed_time: Option<SystemTime>,
    pub queue_state: ListState,
    pub volume_state: SliderState,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            selected_widget: CLMP3Widget::Player,
            last_display_widget: CLMP3Widget::Display,
            volume_changed_time: None,
            queue_state: ListState::default(),
            volume_state: SliderState::new(100.0, 0.0, 100.0),
        }
    }

    pub fn get_selected_widget(&self) -> CLMP3Widget {
        self.selected_widget
    }

    pub fn set_selected_widget(&mut self, val: CLMP3Widget) {
        self.selected_widget = val;
        if val.get_area() == WidgetArea::Display {
            self.last_display_widget = val;
        }
    }

    pub fn get_last_display_widget(&self) -> CLMP3Widget {
        self.last_display_widget
    }

    pub fn set_volume_changed_time(&mut self, val: SystemTime) {
        self.volume_changed_time = Some(val);
    }

    pub fn draw_volume_slider(&self) -> bool {
        if let Some(time) = self.volume_changed_time {
            if time.elapsed().expect("Time error").as_secs() < 1 {
                return true;
            }
        }
        false
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
