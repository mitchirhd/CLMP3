use std::{
    collections::HashMap, path::PathBuf, sync::{Arc, RwLock, mpsc::{self, Sender}}, thread,
};

use image::DynamicImage;
use lofty::{
    config::ParseOptions, file::TaggedFileExt, picture::PictureType, probe::Probe, tag::Accessor,
};

use crate::{enums::Clmp3Event, source_builder::{SourceId, file_name}};

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub pic: Option<DynamicImage>,
}

impl Metadata {
    pub fn new(
        title: String,
        artist: Option<String>,
        album: Option<String>,
        pic: Option<DynamicImage>,
    ) -> Self {
        Self {
            title,
            artist,
            album,
            pic,
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
            .and_then(|t| t.title())
            .map(|t| t.to_string())
            .unwrap_or_else(|| file_name(path));

        let artist = tag.as_ref().and_then(|t| t.artist()).map(|a| a.to_string());

        let album = tag.as_ref().and_then(|t| t.album()).map(|a| a.to_string());

        let pic = tag.as_ref().and_then(|t| {
            t.pictures()
                .iter()
                .find(|p| p.pic_type() == PictureType::CoverFront)
                .or_else(|| t.pictures().first())
        });

        match pic {
            Some(pic) => {
                let dyn_image = image::load_from_memory(pic.data()).ok();
                Metadata::new(title, artist, album, dyn_image)
            }
            None => Metadata::new(title, artist, album, None),
        }
    }
}

pub struct MetadataLoader {
    jobs: mpsc::Sender<(SourceId, PathBuf)>,
}

impl MetadataLoader {
    pub fn new(event_tx: Sender<Clmp3Event>, map: Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>) -> MetadataLoader {
        let (jobs, inbox) = mpsc::channel::<(SourceId, PathBuf)>();

        thread::spawn(move || {
            for (id, path) in inbox {
                if map.read().unwrap().contains_key(&id) {
                    continue;
                }
                let metadata = Metadata::from_file(&path);
                let mut map_write = map.write().unwrap();
                map_write.insert(id, Arc::new(metadata));
                if map_write.len() == 1{
                    let _ = event_tx.send(Clmp3Event::NewSong);
                }
            }
        });

        MetadataLoader { jobs }
    }

    pub fn load(&self, id: SourceId, path: PathBuf) {
        let _ = self.jobs.send((id, path));
    }
}
