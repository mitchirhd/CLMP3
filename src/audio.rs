use rodio::MixerDeviceSink;
use rodio::source::{SineWave, Source};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct AudioControls {
    pub is_stopped: AtomicBool,
    pub is_paused: AtomicBool,
}

pub struct AudioPlayer {
    sink: MixerDeviceSink,
    pub has_source: bool,
    controls: Arc<AudioControls>,
}

impl AudioPlayer {
    pub fn new() -> (AudioPlayer, Arc<AudioControls>) {
        let handle =
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");

        let controls = Arc::new(AudioControls {
            is_stopped: AtomicBool::new(true),
            is_paused: AtomicBool::new(false),
        });

        (
            AudioPlayer {
                sink: handle,
                has_source: true,
                controls: controls.clone(),
            },
            controls,
        )
    }

    pub fn start_stop(&mut self) {
        if self.has_source {
            if self.controls.is_stopped.load(Ordering::Relaxed) {
                self.start();
            } else if self.controls.is_paused.load(Ordering::Relaxed) {
                self.controls.is_paused.store(false, Ordering::Relaxed);
            } else {
                self.controls.is_paused.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn load_sine(&mut self, freq: f32) 
    {

    }

    fn start(&mut self) {
        let controls = self.controls.clone();
        controls.is_stopped.store(false, Ordering::Relaxed);
        let source = SineWave::new(440.0).take_duration(Duration::from_secs(5))
            .amplify(0.20)
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
        self.sink.mixer().add(source);
        self.has_source = true;
    }
}
