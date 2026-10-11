use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        mpsc::{self, Sender},
    },
    thread,
};

use image::DynamicImage;
use lofty::{
    config::ParseOptions, file::TaggedFileExt, picture::PictureType, probe::Probe, tag::Accessor,
};

use crate::{
    enums::Clmp3Event,
    source_builder::{SourceId, file_name},
};

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub path: PathBuf,
    pub source_id: SourceId,
}

impl Metadata {
    pub fn new(
        title: String,
        artist: Option<String>,
        album: Option<String>,
        path: PathBuf,
        source_id: SourceId,
    ) -> Self {
        Self {
            title,
            artist,
            album,
            path,
            source_id,
        }
    }

    pub fn from_file(id: SourceId, path: &PathBuf) -> Metadata {
        let options = ParseOptions::new().read_properties(false);
        let tagged_file = Probe::open(path)
            .and_then(|probe| probe.options(options).read())
            .ok();
        let tag = tagged_file
            .as_ref()
            .and_then(|f| f.primary_tag().or_else(|| f.first_tag()));

        let title = tag
            .and_then(|t| t.title())
            .map(|t| t.to_string())
            .unwrap_or_else(|| file_name(path));

        let artist = tag.as_ref().and_then(|t| t.artist()).map(|a| a.to_string());

        let album = tag.as_ref().and_then(|t| t.album()).map(|a| a.to_string());

        Metadata::new(title, artist, album, path.clone(), id)
    }

    pub fn load_pic(path: &PathBuf) -> Option<DynamicImage> {
        let options = ParseOptions::new().read_properties(false);
        let tagged_file = Probe::open(path)
            .and_then(|probe| probe.options(options).read())
            .ok();
        let tag = tagged_file
            .as_ref()
            .and_then(|f| f.primary_tag().or_else(|| f.first_tag()));

        let pic = tag.as_ref().and_then(|t| {
            t.pictures()
                .iter()
                .find(|p| p.pic_type() == PictureType::CoverFront)
                .or_else(|| t.pictures().first())
        });

        pic.and_then(|pic| image::load_from_memory(pic.data()).ok())
    }
}

pub struct MetadataLoader {
    loader_jobs: mpsc::Sender<(SourceId, PathBuf)>,
    pic_jobs: mpsc::Sender<(SourceId, PathBuf)>,
}

pub type MetadataMap = Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>;

impl MetadataLoader {
    pub fn new(event_tx: Sender<Clmp3Event>, map: MetadataMap) -> MetadataLoader {
        let loader_jobs = Self::spawn_loader_thread(event_tx.clone(), map);
        let pic_jobs = Self::spawn_pic_thread(event_tx);

        MetadataLoader {
            loader_jobs,
            pic_jobs,
        }
    }

    fn spawn_loader_thread(
        event_tx: Sender<Clmp3Event>,
        map: MetadataMap,
    ) -> mpsc::Sender<(SourceId, PathBuf)> {
        let (jobs, inbox) = mpsc::channel::<(SourceId, PathBuf)>();

        thread::spawn(move || {
            for (id, path) in inbox {
                if map.read().unwrap().contains_key(&id) {
                    continue;
                }
                let metadata = Metadata::from_file(id.clone(), &path);
                let mut map_write = map.write().unwrap();
                map_write.insert(id, Arc::new(metadata));
                if map_write.len() == 1 {
                    let _ = event_tx.send(Clmp3Event::NewSongStarted);
                }
            }
        });
        jobs
    }

    fn spawn_pic_thread(event_tx: Sender<Clmp3Event>) -> mpsc::Sender<(SourceId, PathBuf)> {
        let (jobs, inbox) = mpsc::channel::<(SourceId, PathBuf)>();

        thread::spawn(move || {
            while let Ok(mut job) = inbox.recv() {
                while let Ok(newer) = inbox.try_recv() {
                    // skip old entries until arrived at newest
                    job = newer;
                }
                let (id, path) = job;
                let img = Metadata::load_pic(&path);
                if event_tx.send(Clmp3Event::CoverLoaded(id, img)).is_err() {
                    break;
                }
            }
        });

        jobs
    }

    pub fn load(&self, id: SourceId, path: PathBuf) {
        let _ = self.loader_jobs.send((id, path));
    }

    pub fn load_pic(&self, id: SourceId, path: PathBuf) {
        let _ = self.pic_jobs.send((id, path));
    }
}
