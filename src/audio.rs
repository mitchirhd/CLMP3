use rodio::source::{
    Amplify, EmptyCallback, Pausable, Skippable, Source, Stoppable, TrackPosition,
};
use rodio::{MixerDeviceSink, Player};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use crate::app_state::AppState;
use crate::enums::Clmp3Event;
use crate::source_builder::SourceType;

struct SourceQueueContents {
    previous: Vec<SourceType>,
    upcoming: VecDeque<SourceType>,
}

pub struct SourceQueue {
    contents: RwLock<SourceQueueContents>,
}

impl SourceQueue {
    pub fn new() -> SourceQueue {
        SourceQueue {
            contents: RwLock::new(SourceQueueContents {
                previous: Vec::new(),
                upcoming: VecDeque::new(),
            }),
        }
    }

    pub fn enqueue(&self, item: SourceType) {
        self.contents
            .write()
            .expect("RwLock acquired")
            .upcoming
            .push_back(item);
    }

    pub fn current_to_previous(&self) {
        let mut contents = self.contents.write().expect("queue lock poisoned");

        if let Some(item) = contents.upcoming.pop_front() {
            contents.previous.push(item);
        }
    }

    pub fn peek(&self) -> Option<SourceType> {
        self.contents
            .read()
            .expect("RwLock acquired")
            .upcoming
            .front()
            .cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.contents
            .read()
            .expect("RwLock acquired")
            .upcoming
            .is_empty()
    }

    pub fn snapshot(&self) -> (usize, Vec<SourceType>) {
        let contents = self.contents.read().expect("queue lock poisoned");
        let prev_len = contents.previous.len();

        let items = contents
            .previous
            .iter()
            .chain(contents.upcoming.iter())
            .cloned()
            .collect();

        (prev_len, items)
    }

    pub fn prev_len(&self) -> usize {
        self.contents
            .read()
            .expect("RwLock acquired")
            .previous
            .len()
    }

    pub fn replay(&self) {
        let mut contents = self.contents.write().expect("queue lock poisoned");
        if let Some(item) = contents.previous.pop() {
            contents.upcoming.push_front(item);
        }
    }
}

pub struct AudioControls {
    is_stopped: AtomicBool,
    is_paused: AtomicBool,
    volume: Mutex<f32>,
    current_source: Mutex<Option<SourceType>>,
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
            current_source: Mutex::new(None),
            total_duration: Mutex::new(None),
            position: Mutex::new(Duration::ZERO),
            skip_current: AtomicBool::new(false),
            previous: AtomicBool::new(false),
        }
    }

    pub fn reset(&self) {
        self.set_position(Duration::ZERO);
        self.set_current_source(None);
        self.set_skip_current(false);
        self.set_previous_requested(false);
        self.set_total_duration(Duration::ZERO);
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

    pub fn get_current_source(&self) -> Option<SourceType> {
        self.current_source.lock().expect("Mutex acquired").clone()
    }

    pub fn set_current_source(&self, source: Option<SourceType>) {
        *self.current_source.lock().expect("Mutex acquired") = source;
    }

    pub fn get_skip_current(&self) -> bool {
        self.skip_current.load(Ordering::Relaxed)
    }

    pub fn set_skip_current(&self, val: bool) {
        self.skip_current.store(val, Ordering::Relaxed);
        if val {
            self.set_is_paused(false);
        }
    }

    pub fn previous_requested(&self) -> bool {
        self.previous.load(Ordering::Relaxed)
    }

    pub fn set_previous_requested(&self, val: bool) {
        self.previous.store(val, Ordering::Relaxed);
        if val {
            self.set_is_paused(false);
        }
    }
}

pub struct AudioPlayer {
    _sink: MixerDeviceSink,
    player: Player,
    controls: Arc<AudioControls>,
    queue: Arc<SourceQueue>,
    event_tx: Sender<Clmp3Event>,
}

impl AudioPlayer {
    pub fn new(
        event_tx: Sender<Clmp3Event>,
    ) -> (AudioPlayer, Arc<AudioControls>, Arc<SourceQueue>) {
        let mut handle =
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
        handle.log_on_drop(false);

        let player = rodio::Player::connect_new(handle.mixer());

        let controls = Arc::new(AudioControls::new());

        let queue = Arc::new(SourceQueue::new());

        (
            AudioPlayer {
                _sink: handle,
                player,
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
        if self.controls.get_is_stopped() && self.has_next() {
            self.play_next(state);
        } else if self.controls.get_is_paused() {
            self.controls.set_is_paused(false);
            state.update_queue_state();
        } else {
            self.controls.set_is_paused(true);
        }
    }

    pub fn play_next(&mut self, state: &mut AppState) {
        let Some(source) = self.queue.peek() else {
            return;
        };

        let controls = self.controls.clone();
        controls.set_is_stopped(false);
        let source_data = source.build();
        self.controls.set_current_source(Some(source));
        let queue_source = source_data
            .track_position()
            .amplify(1.0)
            .skippable()
            .stoppable()
            .pausable(false)
            .periodic_access(Duration::from_secs_f64(1.0 / 30.0), move |src| {
                Self::control_tick(&controls, src);
            });
        *self.controls.total_duration.lock().expect("Mutex acquired") =
            queue_source.total_duration();
        self.player.append(queue_source);

        let event_tx = self.event_tx.clone();
        let end_callback = EmptyCallback::new(Box::new(move || {
            event_tx.send(Clmp3Event::SongEnded).unwrap();
        }));
        self.player.append(end_callback);
        state.update_queue_state();
    }

    fn control_tick(controls: &AudioControls, src: &mut ControlledSource) {
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

        if controls.previous_requested() {
            if track_pos.get_pos() > Duration::from_secs(1) {
                src.try_seek(Duration::ZERO).unwrap();
                controls.set_previous_requested(false);
            } else {
                src.inner_mut().inner_mut().skip();
            }
        }
    }
}

type Built = Box<dyn rodio::Source<Item = f32> + Send>;
type ControlledSource = Pausable<Stoppable<Skippable<Amplify<TrackPosition<Built>>>>>;
