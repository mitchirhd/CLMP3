use std::{
    fs::{self},
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};

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
            SourceType::File(path) => {
                let bytes = fs::read(path).expect("Could not open file");
                let len = bytes.len() as u64;
                Box::new(
                    Decoder::builder()
                        .with_data(Cursor::new(bytes))
                        .with_byte_len(len)
                        .with_seekable(true)
                        .build()
                        .expect("Could not decode audio"),
                )
            }
        }
    }

    pub fn id(&self) -> Option<SourceId> {
        match self {
            SourceType::SineWave(_, _) => None,
            SourceType::File(path) => Some(SourceId::File(path.clone())),
        }
    }
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum SourceId {
    File(PathBuf),
}
