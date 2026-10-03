use std::{fs::File, path::PathBuf, time::Duration};

use lofty::{file::TaggedFileExt, probe::Probe, tag::Accessor};
use rodio::{Decoder, Source, source::SineWave};

#[derive(Clone)]
pub enum SourceType {
    SineWave(f32, Duration),
    File(PathBuf),
}

impl SourceType {
    pub fn build(&self) -> Box<dyn Source<Item = f32> + Send> {
        match self {
            SourceType::SineWave(freq, duration) => {
                Box::new(SineWave::new(*freq).take_duration(*duration))
            }
            SourceType::File(path) => Box::new(
                Decoder::try_from(File::open(&path).expect("Could not open file"))
                    .expect("Could not decode audio"),
            ),
        }
    }

    pub fn id(&self) -> Option<SourceId> {
        match self {
            SourceType::SineWave(_, _) => None,
            SourceType::File(path) => Some(SourceId::File(path.clone())),
        }
    }
}

pub fn file_name(path: &PathBuf) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum SourceId {
    File(PathBuf),
}

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
        let tag = Probe::open(path)
            .and_then(|probe| probe.read())
            .ok()
            .and_then(|tagged_file| {
                tagged_file
                    .primary_tag()
                    .or_else(|| tagged_file.first_tag())
                    .cloned()
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
