use std::{fs, path::Path, sync::Arc, thread};

use crate::{
    audio::AudioPlayer, metadata::MetadataLoader, source_builder::SourceType,
    source_queue::SharedQueue,
};

pub struct LocalFileHelper {}

impl LocalFileHelper {
    pub fn load_file(queue: SharedQueue, loader: Arc<MetadataLoader>, path: &Path) {
        let path = path.to_path_buf();
        thread::spawn(move || {
            Self::load_file_internal(&queue, &loader, &path);
        });
    }

    fn load_file_internal(queue: &SharedQueue, loader: &Arc<MetadataLoader>, path: &Path) {
        let file = fs::metadata(path).unwrap();
        if !file.is_file() || !AudioPlayer::can_play(path) {
            return;
        }

        let source_type = Arc::new(SourceType::File(path.to_path_buf()));
        queue.enqueue(source_type.clone());
        loader.load(source_type);
    }

    pub fn load_dir(queue: SharedQueue, loader: Arc<MetadataLoader>, start_path: &Path) {
        let start_path = start_path.to_path_buf();
        thread::spawn(move || {
            Self::load_dir_internal(&queue, &loader, &start_path);
        });
    }

    fn load_dir_internal(queue: &SharedQueue, loader: &Arc<MetadataLoader>, start_path: &Path) {
        let Ok(dir) = fs::read_dir(start_path.to_string_lossy().into_owned()) else {
            return;
        };
        for dir_entry in dir {
            let Ok(dir_entry) = dir_entry else {
                continue;
            };
            if dir_entry.file_type().unwrap().is_dir() {
                Self::load_dir_internal(queue, loader, &dir_entry.path());
            } else {
                Self::load_file_internal(queue, loader, &dir_entry.path());
            }
        }
    }
}
