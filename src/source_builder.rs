use std::time::Duration;

use ratatui_explorer::File;
use rodio::{Decoder, Source, source::SineWave};

use std::fs::File as StdFile;

#[derive(Clone)]
pub enum SourceType {
    SineWave(f32, Duration),
    File(File),
}

impl SourceType {
    pub fn build(&self) -> Box<dyn Source<Item = f32> + Send> {
        match self {
            SourceType::SineWave(freq, duration) => {
                Box::new(SineWave::new(*freq).take_duration(*duration))
            }
            SourceType::File(file) => Box::new(
                Decoder::try_from(StdFile::open(&file.path).expect("Could not open file"))
                    .expect("Could not decode audio"),
            ),
        }
    }
}
