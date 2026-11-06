//! Abstract playback deck. The controller drives decks only through this trait,
//! so the state machine is testable without libmpv (see `SimDeck`).

use crate::model::{AudioAdjust, PictureAdjust};

#[derive(Debug, Clone, PartialEq)]
pub struct LoadSpec {
    pub path: String,
    /// seconds
    pub start: f64,
    /// seconds; None = to the end of the file
    pub end: Option<f64>,
    pub is_still: bool,
    pub speed: f64,
    pub audio: AudioAdjust,
    /// dB actually applied (gain + master + normalisation)
    pub gain_db: f64,
    pub picture: PictureAdjust,
    pub audio_device: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DeckStatus {
    /// Current position in seconds (file timeline).
    pub position: f64,
    pub loaded: bool,
    pub paused: bool,
    /// Reached `end` (or end of file) and is holding the last frame.
    pub at_end: bool,
    pub dropped_frames: u64,
    /// Peak level 0..1 per side (approximate; from the deck if available).
    pub level: (f32, f32),
}

pub trait Deck: Send {
    /// Load and cue at `spec.start`, paused, invisible. Returns when the first frame is ready
    /// (or immediately; `status().loaded` tells).
    fn load(&mut self, spec: &LoadSpec) -> Result<(), String>;
    fn unload(&mut self);
    fn play(&mut self);
    fn pause(&mut self);
    fn seek(&mut self, seconds: f64);
    /// 0.0..1.0 fade multiplier applied on top of the cue's own picture/audio settings.
    fn set_fade(&mut self, picture: f64, audio: f64);
    fn set_visible(&mut self, visible: bool);
    fn set_loop(&mut self, enabled: bool);
    /// Re-apply adjustments while loaded (inspector edits).
    fn apply(&mut self, spec: &LoadSpec);
    fn status(&mut self) -> DeckStatus;
}

impl Deck for Box<dyn Deck> {
    fn load(&mut self, spec: &LoadSpec) -> Result<(), String> {
        (**self).load(spec)
    }
    fn unload(&mut self) {
        (**self).unload()
    }
    fn play(&mut self) {
        (**self).play()
    }
    fn pause(&mut self) {
        (**self).pause()
    }
    fn seek(&mut self, seconds: f64) {
        (**self).seek(seconds)
    }
    fn set_fade(&mut self, picture: f64, audio: f64) {
        (**self).set_fade(picture, audio)
    }
    fn set_visible(&mut self, visible: bool) {
        (**self).set_visible(visible)
    }
    fn set_loop(&mut self, enabled: bool) {
        (**self).set_loop(enabled)
    }
    fn apply(&mut self, spec: &LoadSpec) {
        (**self).apply(spec)
    }
    fn status(&mut self) -> DeckStatus {
        (**self).status()
    }
}

/// SimDeck that advances itself with wall-clock time (used when libmpv is missing,
/// so the app still runs and the UI can be rehearsed).
pub struct ClockedSimDeck {
    pub inner: SimDeck,
    last: std::time::Instant,
}

impl Default for ClockedSimDeck {
    fn default() -> Self {
        Self { inner: SimDeck::default(), last: std::time::Instant::now() }
    }
}

impl ClockedSimDeck {
    fn step(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last).as_secs_f64();
        self.last = now;
        self.inner.advance(dt);
    }
}

impl Deck for ClockedSimDeck {
    fn load(&mut self, spec: &LoadSpec) -> Result<(), String> {
        self.step();
        self.inner.load(spec)
    }
    fn unload(&mut self) {
        self.inner.unload()
    }
    fn play(&mut self) {
        self.step();
        self.inner.play()
    }
    fn pause(&mut self) {
        self.step();
        self.inner.pause()
    }
    fn seek(&mut self, seconds: f64) {
        self.step();
        self.inner.seek(seconds)
    }
    fn set_fade(&mut self, picture: f64, audio: f64) {
        self.inner.set_fade(picture, audio)
    }
    fn set_visible(&mut self, visible: bool) {
        self.inner.set_visible(visible)
    }
    fn set_loop(&mut self, enabled: bool) {
        self.inner.set_loop(enabled)
    }
    fn apply(&mut self, spec: &LoadSpec) {
        self.inner.apply(spec)
    }
    fn status(&mut self) -> DeckStatus {
        self.step();
        self.inner.status()
    }
}

/// Deterministic in-memory deck used by tests and by the UI simulator.
#[derive(Debug, Default)]
pub struct SimDeck {
    pub spec: Option<LoadSpec>,
    pub pos: f64,
    pub playing: bool,
    pub visible: bool,
    pub fade: (f64, f64),
    pub looping: bool,
    pub log: Vec<String>,
}

impl SimDeck {
    /// Advance simulated time.
    pub fn advance(&mut self, dt: f64) {
        if let (true, Some(spec)) = (self.playing, &self.spec) {
            self.pos += dt * spec.speed;
            if let Some(end) = spec.end {
                if self.pos >= end {
                    if self.looping {
                        self.pos = spec.start + (self.pos - end);
                    } else {
                        self.pos = end;
                    }
                }
            }
        }
    }
}

impl Deck for SimDeck {
    fn load(&mut self, spec: &LoadSpec) -> Result<(), String> {
        self.log.push(format!("load {}", spec.path));
        self.spec = Some(spec.clone());
        self.pos = spec.start;
        self.playing = false;
        self.visible = false;
        self.looping = false;
        Ok(())
    }
    fn unload(&mut self) {
        self.log.push("unload".into());
        self.spec = None;
        self.playing = false;
        self.visible = false;
    }
    fn play(&mut self) {
        self.playing = true;
    }
    fn pause(&mut self) {
        self.playing = false;
    }
    fn seek(&mut self, seconds: f64) {
        self.pos = seconds;
    }
    fn set_fade(&mut self, picture: f64, audio: f64) {
        self.fade = (picture, audio);
    }
    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    fn set_loop(&mut self, enabled: bool) {
        self.looping = enabled;
    }
    fn apply(&mut self, spec: &LoadSpec) {
        self.spec = Some(spec.clone());
    }
    fn status(&mut self) -> DeckStatus {
        let at_end = match &self.spec {
            Some(s) => s.end.map(|e| self.pos >= e - 1e-9).unwrap_or(false) && !self.looping,
            None => false,
        };
        DeckStatus {
            position: self.pos,
            loaded: self.spec.is_some(),
            paused: !self.playing,
            at_end,
            dropped_frames: 0,
            level: if self.playing { (0.5, 0.45) } else { (0.0, 0.0) },
        }
    }
}
