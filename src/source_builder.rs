use std::time::Duration;

use lofty::{
    file::TaggedFileExt,
    probe::Probe,
    tag::Accessor,
};
use ratatui_explorer::File;
use rodio::{Decoder, Source, source::SineWave};

use std::fs::File as StdFile;

#[derive(Clone)]
pub enum SourceType {
    SineWave(f32, Duration),
    File(File),
}

impl SourceType {
    pub fn build(&self) -> (Box<dyn Source<Item = f32> + Send>, Metadata) {
        match self {
            SourceType::SineWave(freq, duration) => (
                Box::new(SineWave::new(*freq).take_duration(*duration)),
                Metadata::new(String::from("Sinewave"), None, None),
            ),
            SourceType::File(file) => (
                Box::new(
                    Decoder::try_from(StdFile::open(&file.path).expect("Could not open file"))
                        .expect("Could not decode audio"),
                ),
                Metadata::from_file(&file),
            ),
        }
    }
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

    pub fn from_file(file: &File) -> Metadata {
        let tag = Probe::open(&file.path)
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
            .unwrap_or_else(|| file.name.clone());

        let artist = tag.as_ref().and_then(|t| t.artist()).map(|a| a.to_string());

        let album = tag.as_ref().and_then(|t| t.album()).map(|a| a.to_string());

        Metadata::new(title, artist, album)
    }
}
