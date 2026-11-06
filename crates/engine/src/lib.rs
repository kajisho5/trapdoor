//! TRAPDOOR playout engine.
//!
//! - `model`: the `.trapdoor` show file
//! - `controller`: cue state machine (GO / TAKE / PANIC / end actions / A-B decks)
//! - `player`: deck abstraction (+ `SimDeck` for tests)
//! - `mpv`: libmpv deck, loaded at runtime (no link-time dependency)
//! - `analysis`: ffprobe / ffmpeg sidecar (metadata, thumbnails, waveform, loudness)
//! - `remote`: OSC and HTTP control

pub mod analysis;
pub mod controller;
pub mod model;
pub mod mpv;
pub mod player;
pub mod remote;
pub mod timecode;

#[cfg(windows)]
pub mod output_win;

pub use controller::{Command, Controller, PlayoutState};
pub use model::Show;
