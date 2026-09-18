use rodio::source::{EmptyCallback, Source};
use rodio::{MixerDeviceSink, Player};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::enums::CLMP3Event;
use crate::source_builder::SourceType;

pub struct SourceQueue {
    queue: RwLock<VecDeque<SourceType>>,
}

impl SourceQueue {
    pub fn new() -> SourceQueue {
        SourceQueue {
            queue: RwLock::new(VecDeque::new()),
        }
    }

    pub fn enqueue(&self, item: SourceType) {
        self.queue.write().expect("RwLock acquired").push_back(item);
    }

    pub fn dequeue(&self) -> Option<SourceType> {
        self.queue.write().expect("RwLock acquired").pop_front()
    }

    pub fn peek(&self) -> Option<SourceType> {
        self.queue.read().expect("RwLock acquired").front().cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.read().expect("RwLock acquired").is_empty()
    }

    pub fn as_vec(&self) -> Vec<SourceType> {
        self.queue
            .read()
            .expect("RwLock acquired")
            .iter()
            .cloned()
            .collect()
    }
}

pub struct AudioControls {
    pub is_stopped: AtomicBool,
    pub is_paused: AtomicBool,
}

pub struct AudioPlayer {
    sink: MixerDeviceSink,
    player: Player,
    controls: Arc<AudioControls>,
    queue: Arc<SourceQueue>,
    event_tx: Sender<CLMP3Event>,
}

impl AudioPlayer {
    pub fn new(
        event_tx: Sender<CLMP3Event>,
    ) -> (AudioPlayer, Arc<AudioControls>, Arc<SourceQueue>) {
        let handle =
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
        let player = rodio::Player::connect_new(handle.mixer());

        let controls = Arc::new(AudioControls {
            is_stopped: AtomicBool::new(true),
            is_paused: AtomicBool::new(false),
        });

        let queue = Arc::new(SourceQueue::new());

        (
            AudioPlayer {
                sink: handle,
                player: player,
                controls: controls.clone(),
                queue: queue.clone(),
                event_tx,
            },
            controls,
            queue,
        )
    }

    pub fn can_play(path: &Path) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => matches!(
                ext.to_lowercase().as_str(),
                "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a"
            ),
            None => false,
        }
    }

    pub fn has_source(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn start_stop(&mut self) {
        if self.has_source() {
            if self.controls.is_stopped.load(Ordering::Relaxed) {
                self.play_next();
            } else if self.controls.is_paused.load(Ordering::Relaxed) {
                self.controls.is_paused.store(false, Ordering::Relaxed);
            } else {
                self.controls.is_paused.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn play_next(&mut self) {
        let controls = self.controls.clone();
        controls.is_stopped.store(false, Ordering::Relaxed);
        match self.queue.dequeue() {
            Some(source) => {
                let queue_source = source
                    .build()
                    .stoppable()
                    .pausable(false)
                    .skippable()
                    .periodic_access(Duration::from_millis(1 / 30), move |src| {
                        if controls.is_stopped.load(Ordering::Relaxed) {
                            src.inner_mut().inner_mut().stop();
                        } else {
                            let paused = controls.is_paused.load(Ordering::Relaxed);
                            src.inner_mut().set_paused(paused);
                        }
                    });
                self.player.append(queue_source);

                let event_tx = self.event_tx.clone();
                let end_callback = EmptyCallback::new(Box::new(move || {
                    event_tx.send(CLMP3Event::SongEnded);
                }));
                self.player.append(end_callback);
            }
            None => return,
        }
    }

    fn on_song_end() {}
}
