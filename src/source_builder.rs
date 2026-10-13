use std::{
    fs::{self},
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::Context;
use rodio::{Decoder, Source};

#[derive(Clone)]
pub enum SourceType {
    File(PathBuf),
}

impl SourceType {
    pub fn build(&self) -> anyhow::Result<Box<dyn Source<Item = f32> + Send>> {
        match self {
            SourceType::File(path) => {
                let bytes = fs::read(path)
                    .context("Could not open file, was it moved after adding to queue?")?;
                let len = bytes.len() as u64;
                let built = Decoder::builder()
                    .with_data(Cursor::new(bytes))
                    .with_byte_len(len)
                    .with_seekable(true)
                    .build()
                    .context("Could not decode audio")?;
                Ok(Box::new(built))
            }
        }
    }

    pub fn id(&self) -> SourceId {
        match self {
            SourceType::File(path) => SourceId::File(path.clone()),
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
    None,
    File(PathBuf),
}
