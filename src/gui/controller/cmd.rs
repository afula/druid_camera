use std::time::Duration;

use druid::{ImageBuf, Selector};

// Playback state

pub const PLAYBACK_PLAYING: Selector<Duration> = Selector::new("app.playback-playing");
pub const PLAYBACK_PROGRESS: Selector<u64> = Selector::new("app.playback-progress");
pub const PLAYBACK_DURATION: Selector<u64> = Selector::new("app.playback-progress");
pub const PLAYBACK_PAUSING: Selector = Selector::new("app.playback-pausing");
pub const PLAYBACK_RESUMING: Selector = Selector::new("app.playback-resuming");
pub const PLAYBACK_BLOCKED: Selector = Selector::new("app.playback-blocked");
pub const PLAYBACK_STOPPED: Selector = Selector::new("app.playback-stopped");

// Playback control

pub const PLAY: Selector<usize> = Selector::new("app.play-index");
pub const PLAY_PREVIOUS: Selector = Selector::new("app.play-previous");
pub const PLAY_PAUSE: Selector = Selector::new("app.play-pause");
pub const PLAY_RESUME: Selector = Selector::new("app.play-resume");
pub const PLAY_NEXT: Selector = Selector::new("app.play-next");
pub const PLAY_STOP: Selector = Selector::new("app.play-stop");
pub const PLAY_SEEK: Selector<u64> = Selector::new("app.play-seek");
pub const PLAY_VOLUME: Selector<f64> = Selector::new("app.play-volume");
pub const PLAY_RATE: Selector<f64> = Selector::new("app.play-rate");

//Video Frame

pub const VIDEO_FRAME: Selector<ImageBuf> = Selector::new("app.video-frame");
