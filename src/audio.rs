use rodio::source::Source;
use rodio::{DeviceSinkError, MixerDeviceSink, Player};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::enums::Clmp3Event;
use crate::source_queue::SharedQueue;

const RESTART_THRESHOLD: Duration = Duration::from_secs(1);

pub struct AudioPlayer {
    _sink: MixerDeviceSink,
    sender: Sender<Clmp3Event>,
    player: Player,
    pub queue: SharedQueue,
    pub total_duration: Option<Duration>,
    stopped: bool,
    volume: f32,
}

impl AudioPlayer {
    pub fn new(sender: Sender<Clmp3Event>) -> Result<AudioPlayer, DeviceSinkError> {
        let mut handle = rodio::DeviceSinkBuilder::open_default_sink()?;
        handle.log_on_drop(false);

        let player = rodio::Player::connect_new(handle.mixer());

        Ok(AudioPlayer {
            _sink: handle,
            sender,
            player,
            queue: SharedQueue::new(),
            total_duration: None,
            stopped: true,
            volume: 100.0,
        })
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

    pub fn start_stop(&mut self) -> anyhow::Result<()> {
        if self.stopped() && self.has_next() {
            return self.play_front_or_stop();
        } else if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        };
        Ok(())
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
            self.player.get_pos()
        }
    }

    pub fn previous(&mut self) -> anyhow::Result<()> {
        let should_go_previous = if self.stopped {
            self.queue.prev_len() > 0
        } else {
            self.player.get_pos() <= RESTART_THRESHOLD || self.queue.peek().is_none()
        };

        if should_go_previous {
            self.queue.previous_to_current();
            return self.play_front_or_stop();
        } else if !self.stopped {
            self.player.try_seek(Duration::ZERO)?;
        };
        Ok(())
    }

    pub fn skip(&mut self) {
        if self.stopped {
            self.queue.current_to_previous();
        } else {
            self.player.skip_one();
        }
    }

    pub fn clear(&mut self) -> anyhow::Result<()> {
        self.queue.clear();
        self.play_front_or_stop()
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        if !self.stopped && self.player.empty() {
            self.queue.current_to_previous();
            return self.play_front_or_stop();
        };
        anyhow::Result::Ok(())
    }

    fn play_front_or_stop(&mut self) -> anyhow::Result<()> {
        self.player.stop();

        if let Some(source) = self.queue.peek() {
            let source_data = source.build()?;
            self.total_duration = source_data.total_duration();
            self.player.append(source_data);
            self.stopped = false;
        } else {
            self.total_duration = None;
            self.stopped = true;
        }
        let _ = self.sender.send(Clmp3Event::NewSongStarted);
        Ok(())
    }

    pub fn shuffle(&self) {
        self.queue.shuffle_queue(self.stopped);
        if self.stopped{
            let _ = self.sender.send(Clmp3Event::NewSongStarted);
        }
    }
}
