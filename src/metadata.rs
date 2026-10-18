use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
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
    online_helper::OnlineHelper,
    source_builder::{SourceId, SourceType},
};

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub source: Arc<SourceType>,
}

impl Metadata {
    pub fn new(
        title: String,
        artist: Option<String>,
        album: Option<String>,
        source: Arc<SourceType>,
    ) -> Self {
        Self {
            title,
            artist,
            album,
            source,
        }
    }

    fn from_source(source: Arc<SourceType>) -> Metadata {
        let (title, artist, album) = match &*source {
            SourceType::File(path_buf) => Self::from_file(&source, path_buf),
            SourceType::Url(url) => Self::from_url(&source, url),
        };
        Metadata::new(title, artist, album, source)
    }

    pub fn from_file(
        source: &Arc<SourceType>,
        path: &PathBuf,
    ) -> (String, Option<String>, Option<String>) {
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
            .unwrap_or_else(|| source.fallback_label());

        let artist = tag.as_ref().and_then(|t| t.artist()).map(|a| a.to_string());

        let album = tag.as_ref().and_then(|t| t.album()).map(|a| a.to_string());

        (title, artist, album)
    }

    fn from_url(source: &Arc<SourceType>, url: &str) -> (String, Option<String>, Option<String>) {
        let mut args = OnlineHelper::ytdlp_default_args();
        args.push(String::from("--skip-download"));
        args.push(String::from("--no-playlist"));
        args.push(String::from("--print"));
        args.push(String::from("%(title)s\t%(artist)s\t%(album)s"));
        args.push(String::from(url));
        let text = Command::new("yt-dlp")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok());

        let fields: Vec<Option<String>> = text
            .as_deref()
            .unwrap_or_default()
            .trim_end()
            .split('\t')
            .map(Self::clean_ytdlp_str)
            .collect();

        let field = |i: usize| fields.get(i).cloned().flatten();

        let title = field(0).unwrap_or_else(|| source.fallback_label());
        let artist = field(1);
        let album = field(2);

        (title, artist, album)
    }

    fn clean_ytdlp_str(s: &str) -> Option<String> {
        let s = s.trim();
        (!s.is_empty() && s != "NA").then(|| s.to_string())
    }

    fn pic_from_file(path: &PathBuf) -> Option<DynamicImage> {
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

    fn pic_from_url(url: &str) -> Option<DynamicImage> {
        let mut args = OnlineHelper::ytdlp_default_args();
        args.push(String::from("--skip-download"));
        args.push(String::from("--no-playlist"));
        args.push(String::from("--print"));
        args.push(String::from("thumbnail"));
        args.push(String::from(url));
        let out = Command::new("yt-dlp").args(args).output().ok()?;

        if !out.status.success() {
            return None;
        }

        let stdout = String::from_utf8(out.stdout).ok()?;
        let thumb_url = stdout.lines().next()?.trim();
        if thumb_url.is_empty() {
            return None;
        }

        let bytes = ureq::get(thumb_url)
            .call()
            .ok()?
            .body_mut()
            .read_to_vec()
            .ok()?;

        image::load_from_memory(&bytes).ok()
    }
}

pub struct MetadataLoader {
    loader_jobs: mpsc::Sender<Arc<SourceType>>,
    pic_jobs: mpsc::Sender<Arc<SourceType>>,
}

pub type MetadataMap = Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>;

impl MetadataLoader {
    pub fn new(event_tx: Sender<Clmp3Event>, map: MetadataMap) -> MetadataLoader {
        let loader_jobs = Self::spawn_loader_thread(map);
        let pic_jobs = Self::spawn_pic_thread(event_tx);

        MetadataLoader {
            loader_jobs,
            pic_jobs,
        }
    }

    fn spawn_loader_thread(map: MetadataMap) -> mpsc::Sender<Arc<SourceType>> {
        let (jobs, inbox) = mpsc::channel::<Arc<SourceType>>();

        thread::spawn(move || {
            for source in inbox {
                if map.read().unwrap().contains_key(&source.id()) {
                    continue;
                }
                let metadata = Metadata::from_source(source.clone());

                let mut map_write = map.write().unwrap();
                map_write.insert(source.id(), Arc::new(metadata));
            }
        });
        jobs
    }

    fn spawn_pic_thread(event_tx: Sender<Clmp3Event>) -> mpsc::Sender<Arc<SourceType>> {
        let (jobs, inbox) = mpsc::channel::<Arc<SourceType>>();

        thread::spawn(move || {
            while let Ok(mut source) = inbox.recv() {
                while let Ok(newer) = inbox.try_recv() {
                    // skip old entries until arrived at newest
                    source = newer;
                }
                let img = match &*source.clone() {
                    SourceType::File(path_buf) => Metadata::pic_from_file(path_buf),
                    SourceType::Url(url) => Metadata::pic_from_url(url),
                };
                if event_tx.send(Clmp3Event::CoverLoaded(source, img)).is_err() {
                    break;
                }
            }
        });

        jobs
    }

    pub fn load(&self, source: Arc<SourceType>) {
        let _ = self.loader_jobs.send(source);
    }

    pub fn load_pic(&self, source: Arc<SourceType>) {
        let _ = self.pic_jobs.send(source);
    }
}
