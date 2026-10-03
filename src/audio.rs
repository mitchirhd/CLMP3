use rodio::source::{
    Amplify, EmptyCallback, Pausable, Skippable, Source, Stoppable, TrackPosition,
};
use rodio::{MixerDeviceSink, Player};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, RwLock};
use std::time::Duration;

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
    volume_bits: AtomicU32,
    position_ms: AtomicU64,
    skip_current: AtomicBool,
    previous: AtomicBool,
}

impl AudioControls {
    pub fn new() -> AudioControls {
        AudioControls {
            is_stopped: AtomicBool::new(true),
            is_paused: AtomicBool::new(false),
            volume_bits: AtomicU32::new(100.0_f32.to_bits()),
            position_ms: AtomicU64::new(0),
            skip_current: AtomicBool::new(false),
            previous: AtomicBool::new(false),
        }
    }

    pub fn reset(&self) {
        self.set_position(Duration::ZERO);
        self.set_skip_current(false);
        self.set_previous_requested(false);
    }

    pub fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    pub fn set_is_stopped(&self, val: bool) {
        self.is_stopped.store(val, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }

    pub fn set_is_paused(&self, val: bool) {
        self.is_paused.store(val, Ordering::Relaxed);
    }

    pub fn position(&self) -> Duration {
        Duration::from_millis(self.position_ms.load(Ordering::Relaxed))
    }

    pub fn set_position(&self, pos: Duration) {
        self.position_ms
            .store(pos.as_millis() as u64, Ordering::Relaxed);
    }

    pub fn skip_current(&self) -> bool {
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
    pub queue: SourceQueue,
    event_tx: Sender<Clmp3Event>,
    pub total_duration: Option<Duration>,
    pub current_source: Option<SourceType>,
}

impl AudioPlayer {
    pub fn new(event_tx: Sender<Clmp3Event>) -> (AudioPlayer, Arc<AudioControls>) {
        let mut handle =
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
        handle.log_on_drop(false);

        let player = rodio::Player::connect_new(handle.mixer());

        let controls = Arc::new(AudioControls::new());

        (
            AudioPlayer {
                _sink: handle,
                player,
                controls: controls.clone(),
                queue: SourceQueue::new(),
                event_tx,
                total_duration: None,
                current_source: None,
            },
            controls,
        )
    }

    pub fn can_play(path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            ["mp3", "wav", "flac", "aac", "m4a"]
                .iter()
                .any(|x| e.eq_ignore_ascii_case(x))
        })
    }

    pub fn has_next(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn start_stop(&mut self) {
        if self.controls.is_stopped() && self.has_next() {
            self.play_next();
        } else if self.controls.is_paused() {
            self.controls.set_is_paused(false);
        } else {
            self.controls.set_is_paused(true);
        }
    }

    pub fn set_volume(&self, vol: f32){
        self.player.set_volume((vol/100.0).powf(2.0));
    }

    pub fn play_next(&mut self) {
        let Some(source) = self.queue.peek() else {
            return;
        };

        let controls = self.controls.clone();
        controls.set_is_stopped(false);
        let source_data = source.build();
        self.current_source = Some(source);
        let queue_source = source_data
            .track_position()
            .skippable()
            .stoppable()
            .pausable(false)
            .periodic_access(Duration::from_secs_f64(1.0 / 30.0), move |src| {
                Self::control_tick(&controls, src);
            });
        self.total_duration = queue_source.total_duration();
        self.player.append(queue_source);

        let event_tx = self.event_tx.clone();
        let end_callback = EmptyCallback::new(Box::new(move || {
            event_tx.send(Clmp3Event::SongEnded).unwrap();
        }));
        self.player.append(end_callback);
    }

    fn control_tick(controls: &AudioControls, src: &mut ControlledSource) {
        if controls.is_stopped() {
            src.inner_mut().stop();
        } else {
            let paused = controls.is_paused();
            src.set_paused(paused);
        }

        if controls.skip_current() {
            src.inner_mut().inner_mut().skip();
        }

        let track_pos = src.inner_mut().inner_mut().inner_mut();
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
type ControlledSource = Pausable<Stoppable<Skippable<TrackPosition<Built>>>>;
