use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock, mpsc},
    thread,
};

use lofty::{config::ParseOptions, file::TaggedFileExt, probe::Probe, tag::Accessor};

use crate::source_builder::{SourceId, file_name};

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
}

impl Metadata {
    pub fn new(title: String, artist: Option<String>, album: Option<String>) -> Self {
        Self {
            title,
            artist,
            album,
        }
    }

    pub fn from_file(path: &PathBuf) -> Metadata {
        let options = ParseOptions::new().read_properties(false);
        let tagged_file = Probe::open(path)
            .and_then(|probe| probe.options(options).read())
            .ok();
        let tag = tagged_file.as_ref().and_then(|tagged_file| {
            tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
        });

        let title = tag
            .as_ref()
            .and_then(|t| t.title())
            .map(|t| t.to_string())
            .unwrap_or_else(|| file_name(path));

        let artist = tag.as_ref().and_then(|t| t.artist()).map(|a| a.to_string());

        let album = tag.as_ref().and_then(|t| t.album()).map(|a| a.to_string());

        Metadata::new(title, artist, album)
    }
}

pub struct MetadataLoader {
    jobs: mpsc::Sender<(SourceId, PathBuf)>,
}

impl MetadataLoader {
    pub fn new(map: Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>) -> MetadataLoader {
        let (jobs, inbox) = mpsc::channel::<(SourceId, PathBuf)>();

        thread::spawn(move || {
            for (id, path) in inbox {
                if map.read().unwrap().contains_key(&id) {
                    continue;
                }
                let metadata = Metadata::from_file(&path);
                map.write().unwrap().insert(id, Arc::new(metadata));
            }
        });

        MetadataLoader { jobs }
    }

    pub fn load(&self, id: SourceId, path: PathBuf) {
        let _ = self.jobs.send((id, path));
    }
}
