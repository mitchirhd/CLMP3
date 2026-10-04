use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};

use crate::{
    audio::AudioPlayer,
    enums::{Clmp3Widget, WidgetArea},
    metadata::{Metadata, MetadataLoader},
    source_builder::{SourceId, SourceType},
};

pub struct AppState {
    pub player: AudioPlayer,
    selected_widget: Clmp3Widget,
    last_display_widget: Clmp3Widget,
    volume_changed_time: Option<Instant>,
    pub metadata_loader: Arc<MetadataLoader>,
    pub metadata_map: Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>,
}

impl AppState {
    pub fn new(player: AudioPlayer) -> AppState {
        let metadata_map = Arc::new(RwLock::new(HashMap::new()));
        AppState {
            player,
            selected_widget: Clmp3Widget::Player,
            last_display_widget: Clmp3Widget::Display,
            volume_changed_time: None,
            metadata_loader: Arc::new(MetadataLoader::new(metadata_map.clone())),
            metadata_map,
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
            && time.elapsed().as_secs() < 1
        {
            return true;
        }
        false
    }

    pub fn metadata(&self, source: &SourceType) -> Option<Arc<Metadata>> {
        let id = source.id()?;
        self.metadata_map.read().unwrap().get(&id).cloned()
    }

    pub fn current_metadata(&self) -> Option<Arc<Metadata>> {
        if let Some(source) = self.player.queue.peek() {
            self.metadata(&source)
        } else {
            None
        }
    }
}
