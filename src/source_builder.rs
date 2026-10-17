use std::{
    fs::{self},
    io::Cursor,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

use anyhow::{Context, bail};
use rodio::{Decoder, Source};

use crate::online_helper::OnlineHelper;

const FFMPEG_ARGS: [&str; 8] = [
    "-loglevel",
    "error",
    "-i",
    "pipe:0",
    "-vn",
    "-f",
    "mp3",
    "pipe:1",
];

#[derive(Clone)]
pub enum SourceType {
    File(PathBuf),
    Url(String),
}

impl SourceType {
    pub fn build(&self) -> anyhow::Result<Box<dyn Source<Item = f32> + Send>> {
        match self {
            SourceType::File(path) => Self::build_from_path(path),
            SourceType::Url(url) => Self::build_from_url(url),
        }
    }

    fn build_from_path(path: &PathBuf) -> anyhow::Result<Box<dyn Source<Item = f32> + Send>> {
        let bytes =
            fs::read(path).context("Could not open file, was it moved after adding to queue?")?;
        let len = bytes.len() as u64;
        let built = Decoder::builder()
            .with_data(Cursor::new(bytes))
            .with_byte_len(len)
            .with_seekable(true)
            .build()
            .context("Could not decode audio")?;
        Ok(Box::new(built))
    }

    fn build_from_url(url: &str) -> anyhow::Result<Box<dyn Source<Item = f32> + Send>> {
        let mut args = OnlineHelper::ytdlp_default_args();
        args.push(String::from("-q"));
        args.push(String::from("-f"));
        args.push(String::from("bestaudio/best"));
        args.push(String::from("-o"));
        args.push(String::from("-"));
        args.push(String::from(url));
        let mut ytdlp = Command::new("yt-dlp")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to start yt-dlp (is it installed and on PATH?)")?;

        let ytdlp_stdout = ytdlp
            .stdout
            .take()
            .context("yt-dlp stdout was not captured")?;

        let out = Command::new("ffmpeg")
            .args(FFMPEG_ARGS)
            .stdin(Stdio::from(ytdlp_stdout))
            .stderr(Stdio::null())
            .output()
            .context("failed to run ffmpeg (is it installed and on PATH?)")?;

        let ytdlp_status = ytdlp.wait().context("failed to wait for yt-dlp")?;
        if !ytdlp_status.success() {
            bail!("yt-dlp failed with {ytdlp_status}");
        }
        if !out.status.success() || out.stdout.is_empty() {
            bail!("ffmpeg produced no audio ({})", out.status);
        }

        let len = out.stdout.len();
        let built = Decoder::builder()
            .with_data(Cursor::new(out.stdout))
            .with_byte_len(len as u64)
            .with_seekable(true)
            .build()
            .context("Could not decode audio")?;
        Ok(Box::new(built))
    }

    pub fn id(&self) -> SourceId {
        match self {
            SourceType::File(path) => SourceId::File(path.clone()),
            SourceType::Url(url) => SourceId::Url(url.clone()),
        }
    }

    pub fn fallback_label(&self) -> String {
        match self {
            SourceType::File(path_buf) => Self::file_name(path_buf),
            SourceType::Url(url) => url.clone(),
        }
    }

    pub fn file_name(path: &Path) -> String {
        path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum SourceId {
    None,
    File(PathBuf),
    Url(String),
}

pub type BuildResult = anyhow::Result<Box<dyn Source<Item = f32> + Send>>;

pub enum PollResult {
    Idle,
    Pending,
    Stale,
    Ready(BuildResult),
    Failed,
}

struct PendingSource {
    pub source: Arc<SourceType>,
    pub receiver: Receiver<BuildResult>, // dropping it discards the result of the build
}

pub struct SourceBuilder {
    pending: Option<PendingSource>,
}

impl SourceBuilder {
    pub fn new() -> SourceBuilder {
        SourceBuilder { pending: None }
    }

    pub fn loading(&self) -> bool {
        self.pending.is_some()
    }

    pub fn remove_pending(&mut self) {
        self.pending = None;
    }

    pub fn poll(&mut self, front: Option<Arc<SourceType>>) -> PollResult {
        let Some(pending) = &self.pending else {
            return PollResult::Idle;
        };

        if front.is_none_or(|f| !Arc::ptr_eq(&f, &pending.source)) {
            return PollResult::Stale;
        };

        match pending.receiver.try_recv() {
            Ok(result) => PollResult::Ready(result),
            Err(TryRecvError::Empty) => PollResult::Pending,
            Err(TryRecvError::Disconnected) => PollResult::Failed,
        }
    }

    pub fn start_building(&mut self, source: Arc<SourceType>) {
        let (result_tx, result_rx) = mpsc::channel();
        let source_clone = source.clone();
        thread::spawn(move || {
            let _ = result_tx.send(source_clone.build());
        });

        self.pending = Some(PendingSource {
            source,
            receiver: result_rx,
        });
    }
}
