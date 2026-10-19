use std::{
    collections::HashMap,
    sync::{Arc, RwLock, mpsc::Sender},
    time::Instant,
};

use crate::{
    audio::AudioPlayer,
    enums::{Clmp3Error, Clmp3Event, Clmp3Widget, WidgetArea},
    metadata::Metadata,
    metadata_loader::{MetadataLoader, MetadataMap},
    source_builder::SourceType,
};

const DRAW_VOLUME_DURATION_SECS: u64 = 1;
const VIEW_RESET_SECS: u64 = 10;

pub struct AppState {
    pub player: AudioPlayer,
    selected_widget: Clmp3Widget,
    last_display_widget: Clmp3Widget,
    volume_changed_time: Option<Instant>,
    last_input_time: Option<Instant>,
    pub metadata_loader: Arc<MetadataLoader>,
    pub metadata_map: MetadataMap,
    pub last_error: Option<(Clmp3Error, Vec<String>)>,
    pub all_input_to_widget: bool,
}

impl AppState {
    pub fn new(event_tx: Sender<Clmp3Event>, player: AudioPlayer) -> AppState {
        let metadata_map = Arc::new(RwLock::new(HashMap::new()));
        let queue = player.queue.clone();
        AppState {
            player,
            selected_widget: Clmp3Widget::Player,
            last_display_widget: Clmp3Widget::Display,
            volume_changed_time: None,
            last_input_time: None,
            metadata_loader: Arc::new(MetadataLoader::new(event_tx, metadata_map.clone(), queue)),
            metadata_map,
            last_error: None,
            all_input_to_widget: false,
        }
    }

    pub fn selected_widget(&self) -> Clmp3Widget {
        self.selected_widget
    }

    pub fn set_selected_widget(&mut self, val: Clmp3Widget) {
        self.selected_widget = val;
        if val.area() == WidgetArea::Display {
            self.last_display_widget = val;
        }
    }

    pub fn last_display_widget(&self) -> Clmp3Widget {
        self.last_display_widget
    }

    pub fn set_volume_changed_time(&mut self, val: Instant) {
        self.volume_changed_time = Some(val);
    }

    pub fn draw_volume_slider(&self) -> bool {
        if let Some(time) = self.volume_changed_time
            && time.elapsed().as_secs() < DRAW_VOLUME_DURATION_SECS
        {
            return true;
        }
        false
    }

    pub fn set_last_input_time(&mut self, val: Instant) {
        self.last_input_time = Some(val);
    }

    pub fn reset_view_if_idle(&mut self) {
        if let Some(time) = self.last_input_time
            && time.elapsed().as_secs() > VIEW_RESET_SECS
            && !self.all_input_to_widget
        {
            self.last_display_widget = Clmp3Widget::Display;
            self.selected_widget = Clmp3Widget::Player;
            self.all_input_to_widget = false;
        }
    }

    pub fn metadata(&self, source: &SourceType) -> Option<Arc<Metadata>> {
        let id = source.id();
        self.metadata_map.read().unwrap().get(&id).cloned()
    }

    pub fn clear_all_metadata(&self) {
        self.metadata_map.write().unwrap().clear();
    }

    pub fn current_metadata(&self) -> Option<Arc<Metadata>> {
        if let Some(source) = self.player.queue.peek() {
            self.metadata(&source)
        } else {
            None
        }
    }
}
