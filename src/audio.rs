use rodio::source::{EmptyCallback, Source};
use rodio::{MixerDeviceSink, Player};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use crate::app_state::AppState;
use crate::enums::{CLMP3Event, CLMP3Widget};
use crate::source_builder::{Metadata, SourceType};

pub struct SourceQueue {
    previous: RwLock<Vec<SourceType>>,
    queue: RwLock<Vec<SourceType>>,
}

impl SourceQueue {
    pub fn new() -> SourceQueue {
        SourceQueue {
            previous: RwLock::new(Vec::new()),
            queue: RwLock::new(Vec::new()),
        }
    }

    pub fn enqueue(&self, item: SourceType) {
        self.queue.write().expect("RwLock acquired").push(item);
    }

    pub fn current_to_previous(&self) {
        if !self.is_empty() {
            let val = self.queue.write().expect("RwLock acquired").remove(0);
            self.previous
                .write()
                .expect("RwLock acquired")
                .push(val.clone());
        }
    }

    pub fn peek(&self) -> Option<SourceType> {
        self.queue.read().expect("RwLock acquired").first().cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.read().expect("RwLock acquired").is_empty()
    }

    pub fn prev_as_vec(&self) -> Vec<SourceType> {
        self.previous
            .read()
            .expect("RwLock acquired")
            .iter()
            .cloned()
            .collect()
    }

    pub fn as_vec(&self) -> Vec<SourceType> {
        self.queue
            .read()
            .expect("RwLock acquired")
            .iter()
            .cloned()
            .collect()
    }

    pub fn prev_len(&self) -> usize {
        self.previous.read().expect("RwLock acquired").len()
    }

    pub fn replay(&self) {
        let mut queue = self.queue.write().expect("RwLock acquired");
        let mut prev_queue = self.previous.write().expect("RwLock acquired");

        if !prev_queue.is_empty() {
            queue.insert(0, prev_queue.pop().unwrap());
        }
    }
}

pub struct AudioControls {
    is_stopped: AtomicBool,
    is_paused: AtomicBool,
    volume: Mutex<f32>,
    current_metadata: Mutex<Option<Metadata>>,
    total_duration: Mutex<Option<Duration>>,
    position: Mutex<Duration>,
    skip_current: AtomicBool,
    previous: AtomicBool,
}

impl AudioControls {
    pub fn new() -> AudioControls {
        AudioControls {
            is_stopped: AtomicBool::new(true),
            is_paused: AtomicBool::new(false),
            volume: Mutex::new(1.0),
            current_metadata: Mutex::new(None),
            total_duration: Mutex::new(None),
            position: Mutex::new(Duration::ZERO),
            skip_current: AtomicBool::new(false),
            previous: AtomicBool::new(false),
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

    pub fn get_metadata(&self) -> Option<Metadata> {
        self.current_metadata
            .lock()
            .expect("Mutex acquired")
            .clone()
    }

    pub fn set_metadata(&self, metadata: Option<Metadata>) {
        *self.current_metadata.lock().expect("Mutex acquired") = metadata;
    }

    pub fn get_skip_current(&self) -> bool {
        self.skip_current.load(Ordering::Relaxed)
    }

    pub fn set_skip_current(&self, val: bool) {
        self.skip_current.store(val, Ordering::Relaxed);
    }

    pub fn get_previous(&self) -> bool {
        self.previous.load(Ordering::Relaxed)
    }

    pub fn set_previous(&self, val: bool) {
        self.previous.store(val, Ordering::Relaxed);
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
                // "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a"
                "mp3" | "wav" | "flac" | "aac" | "m4a"
            ),
            None => false,
        }
    }

    pub fn has_next(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn start_stop(&mut self, state: &mut AppState) {
        if self.has_next() || !state.audio_controls.get_is_stopped() {
            // falls was in queue, oder aktuell am spielen
            if self.controls.get_is_stopped() && self.has_next() {
                self.play_next(state);
            } else if self.controls.get_is_paused() {
                self.controls.set_is_paused(false);
                self.update_queue_state(state);
            } else {
                self.controls.set_is_paused(true);
            }
        } else {
            state.set_selected_widget(CLMP3Widget::Load);
        }
    }

    pub fn play_next(&mut self, state: &mut AppState) {
        let controls = self.controls.clone();
        controls.set_is_stopped(false);
        match self.queue.peek() {
            Some(source) => {
                let (source_data, metadata) = source.build();
                self.controls.set_metadata(Some(metadata));
                let queue_source = source_data
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

                        if controls.get_skip_current() {
                            src.inner_mut().inner_mut().skip();
                        }

                        let amp = src.inner_mut().inner_mut().inner_mut();
                        amp.set_factor((controls.get_volume() / 100.0).powf(2.0));

                        let track_pos = src.inner_mut().inner_mut().inner_mut().inner_mut();
                        controls.set_position(track_pos.get_pos());

                        if controls.get_previous() {
                            if track_pos.get_pos() > Duration::from_secs(1) {
                                src.try_seek(Duration::ZERO).unwrap();
                                controls.set_previous(false);
                            } else {
                                src.inner_mut().inner_mut().skip();
                            }
                        }
                    });
                *self.controls.total_duration.lock().expect("Mutex acquired") =
                    queue_source.total_duration();
                self.player.append(queue_source);

                let event_tx = self.event_tx.clone();
                let end_callback = EmptyCallback::new(Box::new(move || {
                    event_tx.send(CLMP3Event::SongEnded).unwrap();
                }));
                self.player.append(end_callback);
                self.update_queue_state(state);
            }
            None => return,
        }
    }

    fn update_queue_state(&self, state: &mut AppState) {
        state.queue_state.select(Some(state.queue.prev_len()));
    }
}
