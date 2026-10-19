use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

use image::DynamicImage;
use lofty::{
    config::ParseOptions, file::TaggedFileExt, picture::PictureType, probe::Probe, tag::Accessor,
};

use crate::{online_helper::OnlineHelper, source_builder::SourceType};

#[derive(Default)]
pub struct LoaderSlot {
    pub shutdown: bool,
    pub ytdlp: Option<Child>,
}

pub type SharedLoaderSlot = Arc<Mutex<LoaderSlot>>;

pub struct OnlineData {
    pub url: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub thumbnail_url: Option<String>,
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

    pub fn from_urls(
        urls: &[String],
        slot: &SharedLoaderSlot,
        mut on_item: impl FnMut(OnlineData),
    ) {
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

        let stdin = yt_process.stdin.take();
        let stdout = yt_process.stdout.take();

        {
            let mut s = slot.lock().unwrap();
            if s.shutdown {
                let _ = yt_process.kill();
                let _ = yt_process.wait();
                return;
            }
            s.ytdlp = Some(yt_process);
        }

        // write all urls in stdin and close it
        if let Some(mut stdin) = stdin {
            let _ = stdin.write_all(urls.join("\n").as_bytes());
        }

        // save all data from stdout
        if let Some(stdout) = stdout {
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
        let child = slot.lock().unwrap().ytdlp.take();
        if let Some(mut child) = child {
            let _ = child.wait(); // reaps the process
        }
    }

    fn clean_ytdlp_str(s: &str) -> Option<String> {
        let s = s.trim();
        (!s.is_empty() && s != "NA").then(|| s.to_string())
    }

    pub fn pic_from_file(path: &PathBuf) -> Option<DynamicImage> {
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

    pub fn pic_from_url(url: &str, thumb: Option<String>) -> Option<DynamicImage> {
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
