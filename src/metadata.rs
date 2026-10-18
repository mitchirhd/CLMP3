use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
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

struct OnlineData {
    url: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    thumbnail_url: Option<String>,
}

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub source: Arc<SourceType>,
    pub thumbnail_url: Option<String>,
}

impl Metadata {
    pub fn new(
        title: String,
        artist: Option<String>,
        album: Option<String>,
        source: Arc<SourceType>,
        thumbnail_url: Option<String>,
    ) -> Self {
        Self {
            title,
            artist,
            album,
            source,
            thumbnail_url,
        }
    }

    pub fn from_file(source: Arc<SourceType>, path: &PathBuf) -> Metadata {
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

        Metadata::new(title, artist, album, source, None)
    }

    fn from_urls(urls: &[String], mut on_item: impl FnMut(OnlineData)) {
        let mut args = OnlineHelper::ytdlp_default_args();
        args.extend(
            [
                "--skip-download",
                "--no-playlist",
                "--ignore-errors",
                "-a",
                "-",
                "--print",
                "%(original_url)s\t%(title)s\t%(artist)s\t%(album)s\t%(thumbnail)s",
            ]
            .map(String::from),
        );

        // create yt-dlp process
        let Ok(mut yt_process) = Command::new("yt-dlp")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            return;
        };

        // write all urls in stdin and close it
        if let Some(mut stdin) = yt_process.stdin.take() {
            let _ = stdin.write_all(urls.join("\n").as_bytes());
        }

        // save all data from stdout
        if let Some(stdout) = yt_process.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let data: Vec<Option<String>> =
                    line.split('\t').map(Self::clean_ytdlp_str).collect();
                let Some(Some(url)) = data.first().cloned() else {
                    continue;
                };
                let field = |i: usize| data.get(i).cloned().flatten();
                on_item(OnlineData {
                    url,
                    title: field(1),
                    artist: field(2),
                    album: field(3),
                    thumbnail_url: field(4),
                });
            }
        }
        let _ = yt_process.wait();
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

    fn pic_from_url(url: &str, thumb: Option<String>) -> Option<DynamicImage> {
        let thumb_url = thumb.or_else(|| Self::thumbnail_url_via_ytdlp(url))?;

        let bytes = ureq::get(thumb_url)
            .call()
            .ok()?
            .body_mut()
            .read_to_vec()
            .ok()?;

        image::load_from_memory(&bytes).ok()
    }

    fn thumbnail_url_via_ytdlp(url: &str) -> Option<String> {
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
        Some(stdout.lines().next()?.trim().to_string())
    }
}

pub struct MetadataLoader {
    loader_jobs: mpsc::Sender<Vec<Arc<SourceType>>>,
    pic_jobs: mpsc::Sender<Arc<SourceType>>,
}

pub type MetadataMap = Arc<RwLock<HashMap<SourceId, Arc<Metadata>>>>;

impl MetadataLoader {
    pub fn new(event_tx: Sender<Clmp3Event>, map: MetadataMap) -> MetadataLoader {
        let loader_jobs = Self::spawn_loader_thread(map.clone());
        let pic_jobs = Self::spawn_pic_thread(event_tx, map);

        MetadataLoader {
            loader_jobs,
            pic_jobs,
        }
    }

    fn spawn_loader_thread(map: MetadataMap) -> mpsc::Sender<Vec<Arc<SourceType>>> {
        let (jobs, inbox) = mpsc::channel::<Vec<Arc<SourceType>>>();

        thread::spawn(move || {
            for batch in inbox {
                let online_data = Self::handle_local_and_get_online_data(batch, &map);
                Self::handle_online_sources(online_data, &map);
            }
        });
        jobs
    }

    fn handle_local_and_get_online_data(
        batch: Vec<Arc<SourceType>>,
        map: &MetadataMap,
    ) -> (Vec<String>, HashMap<String, Arc<SourceType>>) {
        let mut urls: Vec<String> = Vec::new();
        let mut pending: HashMap<String, Arc<SourceType>> = HashMap::new(); // need to remember which source belongs to which url

        for source in batch {
            if map.read().unwrap().contains_key(&source.id()) {
                continue;
            }
            match &*source {
                SourceType::File(path_buf) => {
                    let m = Metadata::from_file(source.clone(), path_buf);
                    map.write().unwrap().insert(source.id(), Arc::new(m));
                }
                SourceType::Url(url) => {
                    if pending.insert(url.clone(), source.clone()).is_none() {
                        urls.push(url.clone());
                    }
                }
            }
        }
        (urls, pending)
    }

    fn handle_online_sources(
        online_data: (Vec<String>, HashMap<String, Arc<SourceType>>),
        map: &MetadataMap,
    ) {
        let (urls, pending) = online_data;

        if urls.is_empty() {
            return; // only local files were in this batch
        }

        Metadata::from_urls(&urls, |data| {
            if let Some(source) = pending.get(&data.url) {
                let meta = Metadata::new(
                    data.title.unwrap_or_else(|| source.fallback_label()),
                    data.artist,
                    data.album,
                    source.clone(),
                    data.thumbnail_url,
                );
                map.write().unwrap().insert(source.id(), Arc::new(meta));
            }
        });

        // online sources that didn't get added to map yet need to use fallback data
        for url in &urls {
            let source = &pending[url];
            let missing = !map.read().unwrap().contains_key(&source.id());
            if missing {
                let meta = Metadata::new(source.fallback_label(), None, None, source.clone(), None);
                map.write().unwrap().insert(source.id(), Arc::new(meta));
            }
        }
    }

    fn spawn_pic_thread(
        event_tx: Sender<Clmp3Event>,
        map: MetadataMap,
    ) -> mpsc::Sender<Arc<SourceType>> {
        let (jobs, inbox) = mpsc::channel::<Arc<SourceType>>();

        thread::spawn(move || {
            while let Ok(mut source) = inbox.recv() {
                while let Ok(newer) = inbox.try_recv() {
                    // skip old entries until arrived at newest
                    source = newer;
                }
                let img = match &*source.clone() {
                    SourceType::File(path_buf) => Metadata::pic_from_file(path_buf),
                    SourceType::Url(url) => {
                        let thumb = map
                            .read()
                            .unwrap()
                            .get(&source.id())
                            .and_then(|m| m.thumbnail_url.clone());
                        Metadata::pic_from_url(url, thumb)
                    }
                };
                if event_tx.send(Clmp3Event::CoverLoaded(source, img)).is_err() {
                    break;
                }
            }
        });

        jobs
    }

    pub fn load(&self, source: Arc<SourceType>) {
        self.load_many(vec![source]);
    }

    pub fn load_many(&self, sources: Vec<Arc<SourceType>>) {
        let _ = self.loader_jobs.send(sources);
    }

    pub fn load_pic(&self, source: Arc<SourceType>) {
        let _ = self.pic_jobs.send(source);
    }
}
