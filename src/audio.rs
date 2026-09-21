use rodio::source::{EmptyCallback, Source};
use rodio::{MixerDeviceSink, Player};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
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
    is_stopped: AtomicBool,
    is_paused: AtomicBool,
    volume: Mutex<f32>,
    total_duration: Mutex<Option<Duration>>,
    position: Mutex<Duration>,
}

impl AudioControls {
    pub fn new() -> AudioControls {
        AudioControls {
            is_stopped: AtomicBool::new(true),
            is_paused: AtomicBool::new(false),
            volume: Mutex::new(1.0),
            total_duration: Mutex::new(None),
            position: Mutex::new(Duration::ZERO),
        }
    }

    pub fn get_is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    pub fn set_is_stopped(&self, val: bool) {
        self.is_stopped.store(val, Ordering::Relaxed);
    }

    pub fn get_is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }

    pub fn set_is_paused(&self, val: bool) {
        self.is_paused.store(val, Ordering::Relaxed);
    }

    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().expect("Mutex acquired")
    }

    pub fn set_volume(&self, val: f32) {
        *self.volume.lock().expect("Mutex acquired") = val;
    }

    pub fn get_position(&self) -> Duration {
        *self.position.lock().expect("Mutex acquired")
    }

    pub fn set_position(&self, pos: Duration) {
        *self.position.lock().expect("Mutex acquired") = pos;
    }

    pub fn get_total_duration(&self) -> Option<Duration> {
        *self.total_duration.lock().expect("Mutex acquired")
    }

    pub fn set_total_duration(&self, duration: Duration) {
        *self.total_duration.lock().expect("Mutex acquired") = Some(duration);
    }
}

pub struct AudioPlayer {
    _sink: MixerDeviceSink,
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

        let controls = Arc::new(AudioControls::new());

        let queue = Arc::new(SourceQueue::new());

        (
            AudioPlayer {
                _sink: handle,
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
        match path.extension() {
            Some(ext) => matches!(
                ext.to_str().unwrap_or_default(),
                "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a"
            ),
            None => false,
        }
    }

    pub fn has_next(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn start_stop(&mut self) {
        if self.controls.get_is_stopped() && self.has_next() {
            self.play_next();
        } else {
            self.controls.set_is_paused(!self.controls.get_is_paused());
        }
    }

    pub fn play_next(&mut self) {
        let controls = self.controls.clone();
        controls.set_is_stopped(false);
        match self.queue.dequeue() {
            Some(source) => {
                let queue_source = source
                    .build()
                    .track_position()
                    .amplify(1.0)
                    .skippable()
                    .stoppable()
                    .pausable(false)
                    .periodic_access(Duration::from_millis(1 / 30), move |src| {
                        if controls.get_is_stopped() {
                            src.inner_mut().stop();
                        } else {
                            let paused = controls.get_is_paused();
                            src.set_paused(paused);
                        }

                        let amp = src.inner_mut().inner_mut().inner_mut();
                        amp.set_factor((controls.get_volume() / 100.0).powf(2.0));

                        let track_pos = src.inner_mut().inner_mut().inner_mut().inner_mut();
                        controls.set_position(track_pos.get_pos());
                    });
                *self.controls.total_duration.lock().expect("Mutex acquired") = queue_source.total_duration();
                self.player.append(queue_source);

                let event_tx = self.event_tx.clone();
                let end_callback = EmptyCallback::new(Box::new(move || {
                    event_tx.send(CLMP3Event::SongEnded).unwrap();
                }));
                self.player.append(end_callback);
            }
            None => return,
        }
    }
}
