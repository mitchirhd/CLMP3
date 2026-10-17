use anyhow::{Context, bail};
use rodio::source::Source;
use rodio::{DeviceSinkError, MixerDeviceSink, Player};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::enums::Clmp3Event;
use crate::source_builder::{BuildResult, PollResult, SourceBuilder};
use crate::source_queue::SharedQueue;

const RESTART_THRESHOLD: Duration = Duration::from_secs(1);
const SCRUB_STEP_MILLIS: Duration = Duration::from_millis(1000);

pub struct AudioPlayer {
    _sink: MixerDeviceSink,
    sender: Sender<Clmp3Event>,
    player: Player,
    pub source_builder: SourceBuilder,
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
            source_builder: SourceBuilder::new(),
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
            self.play_front_or_stop();
        } else if self.source_builder.loading() {
            return Ok(());
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
            self.source_builder.loading()
                || self.player.get_pos() <= RESTART_THRESHOLD
                || self.queue.peek().is_none()
        };

        if should_go_previous {
            self.queue.previous_to_current();
            if !self.stopped {
                self.play_front_or_stop();
            } else {
                let _ = self.sender.send(Clmp3Event::NewSongStarted);
            }
        } else if !self.stopped {
            self.player.try_seek(Duration::ZERO)?;
        };
        Ok(())
    }

    pub fn skip(&mut self) {
        if self.stopped {
            self.queue.current_to_previous();
            let _ = self.sender.send(Clmp3Event::NewSongStarted);
        } else if self.source_builder.loading() {
            self.queue.current_to_previous();
            self.play_front_or_stop();
        } else {
            self.player.skip_one();
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.player.stop();
        self.source_builder.remove_pending();
        self.total_duration = None;
        self.stopped = true;
        let _ = self.sender.send(Clmp3Event::NewSongStarted);
    }

    pub fn scrub(&self, backwards: bool) -> anyhow::Result<()> {
        if self.source_builder.loading() {
            return Ok(());
        }

        let curr_pos = self.player.get_pos();
        let mut scrub_pos = if backwards {
            curr_pos - SCRUB_STEP_MILLIS
        } else {
            curr_pos + SCRUB_STEP_MILLIS
        };

        if let Some(total_dur) = self.total_duration {
            scrub_pos = scrub_pos.clamp(Duration::ZERO, total_dur);
        }
        self.player
            .try_seek(scrub_pos)
            .context("Scrubbing is not supported by source")?;
        Ok(())
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        match self.source_builder.poll(self.queue.peek()) {
            PollResult::Idle => {}
            PollResult::Pending => return Ok(()),
            PollResult::Stale => {
                self.play_front_or_stop(); // current song was moved or deleted while loading
                return Ok(());
            }
            PollResult::Ready(source) => return self.start_built(source),
            PollResult::Failed => {
                self.stop_at_current();
                bail!("Loading the song failed unexpectedly")
            }
        }

        if !self.stopped && self.player.empty() {
            self.queue.current_to_previous();
            self.play_front_or_stop();
        };
        Ok(())
    }

    fn play_front_or_stop(&mut self) {
        self.player.stop();
        self.source_builder.remove_pending();
        self.total_duration = None;

        if let Some(source) = self.queue.peek() {
            self.source_builder.start_building(source);
            self.stopped = false;
        } else {
            self.stopped = true;
            let _ = self.sender.send(Clmp3Event::NewSongStarted);
        }
    }

    fn start_built(&mut self, result: BuildResult) -> anyhow::Result<()> {
        self.source_builder.remove_pending();
        match result {
            Ok(source_data) => {
                self.total_duration = source_data.total_duration();
                self.player.append(source_data);
                let _ = self.sender.send(Clmp3Event::NewSongStarted);
                Ok(())
            }
            Err(e) => {
                self.stop_at_current();
                Err(e)
            }
        }
    }

    fn stop_at_current(&mut self) {
        self.player.stop();
        self.source_builder.remove_pending();
        self.total_duration = None;
        self.stopped = true;
        let _ = self.sender.send(Clmp3Event::NewSongStarted);
    }

    pub fn shuffle(&self) {
        self.queue.shuffle_queue(self.stopped);
        if self.stopped {
            let _ = self.sender.send(Clmp3Event::NewSongStarted);
        }
    }
}
