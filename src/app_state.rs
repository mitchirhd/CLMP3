use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
    time::SystemTime,
};

use ratatui::widgets::ListState;
use tui_slider::SliderState;

use crate::{
    audio::{AudioControls, SourceQueue},
    enums::{Clmp3Widget, WidgetArea},
    source_builder::{Metadata, SourceId, SourceType},
};

pub struct AppState {
    pub audio_controls: Arc<AudioControls>,
    pub queue: Arc<SourceQueue>,
    selected_widget: Clmp3Widget,
    last_display_widget: Clmp3Widget,
    volume_changed_time: Option<SystemTime>,
    pub queue_state: ListState,
    pub volume_state: SliderState,
    pub metadata_map: Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>,
}

impl AppState {
    pub fn new(audio_controls: Arc<AudioControls>, queue: Arc<SourceQueue>) -> AppState {
        let val = AppState {
            audio_controls,
            queue,
            selected_widget: Clmp3Widget::Player,
            last_display_widget: Clmp3Widget::Display,
            volume_changed_time: None,
            queue_state: ListState::default(),
            volume_state: SliderState::new(100.0, 0.0, 100.0),
            metadata_map: Arc::new(RwLock::new(HashMap::new())),
        };
        val.audio_controls
            .set_volume(val.volume_state.value() as f32);
        val
    }

    pub fn get_selected_widget(&self) -> Clmp3Widget {
        self.selected_widget
    }

    pub fn set_selected_widget(&mut self, val: Clmp3Widget) {
        self.selected_widget = val;
        if val.area() == WidgetArea::Display {
            self.last_display_widget = val;
        }
        if val != Clmp3Widget::Queue {
            self.queue_state.select(None);
            self.update_queue_state();
        }
    }

    pub fn get_last_display_widget(&self) -> Clmp3Widget {
        self.last_display_widget
    }

    pub fn set_volume_changed_time(&mut self, val: SystemTime) {
        self.volume_changed_time = Some(val);
    }

    pub fn draw_volume_slider(&self) -> bool {
        if let Some(time) = self.volume_changed_time
            && time.elapsed().expect("Time error").as_secs() < 1
        {
            return true;
        }
        false
    }

    pub fn get_metadata(&self, source: &SourceType) -> Option<Arc<Metadata>> {
        let id = source.id()?;
        self.metadata_map.read().unwrap().get(&id).cloned()
    }

    pub fn get_current_metadata(&self) -> Option<Arc<Metadata>> {
        if let Some(source) = self.audio_controls.get_current_source() {
            self.get_metadata(&source)
        } else {
            None
        }
    }

    pub fn spawn_metadata_load(
        &self,
        id: SourceId,
        load: impl FnOnce() -> Metadata + Send + 'static,
    ) {
        if self
            .metadata_map
            .read()
            .expect("metadata map lock poisoned")
            .contains_key(&id)
        {
            return;
        }

        let map = Arc::clone(&self.metadata_map);
        thread::spawn(move || {
            let metadata = load();
            let mut map = map.write().expect("metadata map lock poisoned");
            map.insert(id, Arc::new(metadata));
        });
    }

    pub fn update_queue_state(&mut self) {
        *self.queue_state.offset_mut() = self.queue.prev_len().saturating_sub(5)
    }
}
