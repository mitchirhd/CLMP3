use rodio::source::Source;
use rodio::{MixerDeviceSink, Player};
use std::path::Path;
use std::time::Duration;

use crate::source_queue::SourceQueue;

const RESTART_THRESHOLD: Duration = Duration::from_secs(1);

pub struct AudioPlayer {
    _sink: MixerDeviceSink,
    player: Player,
    pub queue: SourceQueue,
    pub total_duration: Option<Duration>,
    stopped: bool,
    volume: f32,
}

impl AudioPlayer {
    pub fn new() -> AudioPlayer {
        let mut handle =
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
        handle.log_on_drop(false);

        let player = rodio::Player::connect_new(handle.mixer());

        AudioPlayer {
            _sink: handle,
            player,
            queue: SourceQueue::new(),
            total_duration: None,
            stopped: true,
            volume: 100.0,
        }
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
        if self.stopped() && self.has_next() {
            self.play_front_or_stop();
        } else if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    pub fn paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn stopped(&self) -> bool {
        self.stopped
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 100.0);
        self.player.set_volume((self.volume / 100.0).powf(2.0));
    }

    pub fn position(&self) -> Duration {
        if let Some(total) = self.total_duration {
            self.player.get_pos().min(total) // total duration is updated one tick before player.position
        } else {
            Duration::ZERO
        }
    }

    pub fn set_position(&self, pos: Duration) {
        self.player.try_seek(pos).unwrap();
    }

    pub fn previous(&mut self) {
        if self.stopped {
            return;
        }
        if self.player.get_pos() <= RESTART_THRESHOLD {
            self.queue.previous_to_current();
            self.play_front_or_stop();
        } else {
            self.player.try_seek(Duration::ZERO).unwrap();
        }
    }

    pub fn skip(&mut self) {
        self.player.skip_one();
    }

    pub fn update(&mut self) {
        if !self.stopped && self.player.empty() {
            self.queue.current_to_previous();
            self.play_front_or_stop();
        }
    }

    fn play_front_or_stop(&mut self) {
        self.player.stop();

        if let Some(source) = self.queue.peek() {
            let source_data = source.build();
            self.total_duration = source_data.total_duration();
            self.player.append(source_data);
            self.stopped = false;
            return;
        }

        self.total_duration = None;
        self.stopped = true;
    }
}
