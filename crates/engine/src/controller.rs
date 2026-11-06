//! Cue state machine. Owns the show, two decks (A/B) and decides what is on air.
//!
//! The controller is single-threaded: the app calls `handle()` for commands and `tick(now)`
//! about 60 times a second. All decisions depend only on those calls and deck status,
//! so the logic is tested with `SimDeck`.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::model::*;
use crate::player::{Deck, LoadSpec};
use crate::timecode::format_frames;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Command {
    Go,
    Take { cue_id: String },
    TakeNumber { number: String },
    Standby { cue_id: String },
    StandbyNumber { number: String },
    StandbyNext,
    StandbyPrev,
    Pause,
    Resume,
    TogglePause,
    Stop,
    Panic,
    /// Jump so that `seconds` remain.
    GotoRemain { seconds: f64 },
    /// Seek within the on-air cue, 0..1 of its duration.
    Scrub { ratio: f64 },
    ToggleLoop,
    Black { on: bool },
    ToggleBlack,
    Bank { slot: String },
    Lock { on: bool },
    // --- editing (refused while locked) ---
    AddCue { cue: Cue, after: Option<String> },
    UpdateCue { cue: Cue },
    DeleteCue { cue_id: String },
    MoveCue { cue_id: String, to_index: usize },
    SetBank { slot: String, cue_id: Option<String> },
    UpdateShow { settings: ShowSettings },
    UpsertMedia { media: Media },
    LoadShow { show: Box<Show> },
}

impl Command {
    pub fn is_edit(&self) -> bool {
        matches!(
            self,
            Command::AddCue { .. }
                | Command::UpdateCue { .. }
                | Command::DeleteCue { .. }
                | Command::MoveCue { .. }
                | Command::SetBank { .. }
                | Command::UpdateShow { .. }
                | Command::LoadShow { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Playing,
    Paused,
    /// Holding the last frame (end action Hold) or a still with no duration.
    Hold,
    /// Waiting at the first frame (start action PauseAtStart).
    Cued,
    FadingOut,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgramState {
    pub cue_id: String,
    pub number: String,
    pub name: String,
    pub phase: Phase,
    pub end: EndAction,
    pub looping: bool,
    pub elapsed_frames: u64,
    pub remain_frames: Option<u64>,
    pub duration_frames: Option<u64>,
    /// 0..1 progress, if the duration is known.
    pub progress: Option<f64>,
    pub elapsed_tc: String,
    pub remain_tc: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub t: f64,
    pub level: String,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayoutState {
    pub program: Option<ProgramState>,
    pub standby_cue_id: Option<String>,
    /// The standby cue is loaded on a deck and parked at its first frame.
    pub standby_ready: bool,
    pub black: bool,
    pub locked: bool,
    pub dropped_frames: u64,
    pub level: (f32, f32),
    pub show_rev: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum After {
    Nothing,
    /// Hide and unload the deck, clear program if it is on air.
    Clear,
}

#[derive(Debug, Clone, Copy)]
struct Ramp {
    deck: usize,
    from: f64,
    to: f64,
    t0: f64,
    dur: f64,
    after: After,
}

#[derive(Debug, Clone)]
struct OnAir {
    deck: usize,
    cue_id: String,
    phase: Phase,
    loops_done: u32,
    looping_override: Option<bool>,
    fade_out_started: bool,
}

#[derive(Debug, Clone)]
struct Pending {
    deck: usize,
    cue_id: String,
    fade_in: f64,
}

pub struct Controller<D: Deck> {
    pub show: Show,
    pub decks: [D; 2],
    /// cue id loaded per deck
    loaded: [Option<String>; 2],
    program: Option<OnAir>,
    pending: Option<Pending>,
    standby: Option<String>,
    ramps: Vec<Ramp>,
    fade_level: [f64; 2],
    black: bool,
    locked: bool,
    now: f64,
    show_rev: u64,
    pub log: VecDeque<LogEntry>,
}

const PANIC_SECONDS: f64 = 1.0;

impl<D: Deck> Controller<D> {
    pub fn new(show: Show, decks: [D; 2]) -> Self {
        let mut c = Self {
            standby: show.cues.first().map(|c| c.id.clone()),
            show,
            decks,
            loaded: [None, None],
            program: None,
            pending: None,
            ramps: vec![],
            fade_level: [1.0, 1.0],
            black: false,
            locked: false,
            now: 0.0,
            show_rev: 1,
            log: VecDeque::new(),
        };
        c.preload();
        c
    }

    fn info(&mut self, msg: impl Into<String>) {
        self.push_log("info", msg.into());
    }
    fn warn(&mut self, msg: impl Into<String>) {
        self.push_log("warn", msg.into());
    }
    fn push_log(&mut self, level: &str, msg: String) {
        self.log.push_back(LogEntry { t: self.now, level: level.into(), msg });
        while self.log.len() > 500 {
            self.log.pop_front();
        }
    }

    pub fn locked(&self) -> bool {
        self.locked
    }
    pub fn show_rev(&self) -> u64 {
        self.show_rev
    }

    fn rate(&self) -> crate::timecode::FrameRate {
        self.show.show.frame_rate
    }

    fn spec_for(&self, cue: &Cue) -> Option<LoadSpec> {
        let media = cue.media_id.as_deref().and_then(|id| self.show.media(id))?;
        let rate = self.rate();
        let is_still = media.kind == MediaKind::Still;
        let end = if is_still { None } else { self.show.cue_out(cue).map(|o| rate.frames_to_seconds(o)) };
        Some(LoadSpec {
            path: media.path.clone(),
            start: if is_still { 0.0 } else { rate.frames_to_seconds(cue.in_frame) },
            end,
            is_still,
            speed: cue.speed.clamp(0.25, 2.0),
            audio: cue.audio.clone(),
            gain_db: self.show.applied_gain_db(cue),
            picture: cue.picture.clone(),
            audio_device: self.show.show.audio.device.clone(),
        })
    }

    fn cue(&self, id: &str) -> Option<&Cue> {
        self.show.cues.iter().find(|c| c.id == id)
    }

    fn program_deck(&self) -> Option<usize> {
        self.program.as_ref().map(|p| p.deck)
    }

    fn busy(&self, deck: usize) -> bool {
        self.program_deck() == Some(deck)
            || self.pending.as_ref().map(|p| p.deck) == Some(deck)
            || self.ramps.iter().any(|r| r.deck == deck)
    }

    fn free_deck(&self) -> Option<usize> {
        (0..2).find(|&d| !self.busy(d))
    }

    /// Load the standby cue into a free deck, parked at its first frame.
    fn preload(&mut self) {
        let Some(sb) = self.standby.clone() else { return };
        if self.loaded.iter().enumerate().any(|(d, l)| l.as_deref() == Some(&sb) && !self.busy(d)) {
            return;
        }
        let Some(d) = self.free_deck() else { return };
        self.load_into(d, &sb);
    }

    fn load_into(&mut self, d: usize, cue_id: &str) -> bool {
        let Some(cue) = self.cue(cue_id).cloned() else { return false };
        let Some(spec) = self.spec_for(&cue) else {
            self.warn(format!("cue {} has no playable media", cue.number));
            return false;
        };
        match self.decks[d].load(&spec) {
            Ok(()) => {
                self.decks[d].set_visible(false);
                self.decks[d].set_fade(1.0, 1.0);
                self.fade_level[d] = 1.0;
                self.loaded[d] = Some(cue_id.to_string());
                true
            }
            Err(e) => {
                self.warn(format!("load failed for cue {}: {e}", cue.number));
                self.loaded[d] = None;
                false
            }
        }
    }

    fn next_cue_after(&self, id: &str) -> Option<String> {
        let i = self.show.cue_index(id)?;
        self.show.cues.get(i + 1).map(|c| c.id.clone())
    }

    fn set_standby(&mut self, id: Option<String>) {
        if self.standby == id {
            return;
        }
        // Free a parked deck holding the old standby (not on air).
        for d in 0..2 {
            if !self.busy(d) && self.loaded[d].is_some() && self.loaded[d] != id {
                self.decks[d].unload();
                self.loaded[d] = None;
            }
        }
        self.standby = id;
        self.preload();
    }

    fn take(&mut self, cue_id: &str) {
        let Some(cue) = self.cue(cue_id).cloned() else {
            self.warn("take: unknown cue");
            return;
        };
        // Find or load a deck for this cue.
        let deck = match (0..2).find(|&d| self.loaded[d].as_deref() == Some(cue_id) && !self.busy(d)) {
            Some(d) => d,
            None => {
                // Taking the cue that is on air again: restart on the other deck.
                let target = match self.free_deck() {
                    Some(d) => d,
                    None => {
                        // Both busy (mid transition): cut everything that is fading.
                        self.ramps.clear();
                        self.pending = None;
                        1 - self.program_deck().unwrap_or(1)
                    }
                };
                if !self.load_into(target, cue_id) {
                    return;
                }
                target
            }
        };

        let rate = self.rate();
        let dip = cue.transition.kind == TransitionKind::Dissolve && cue.transition.duration_ms > 0;
        let fade_in = cue.audio.fade_in_ms as f64 / 1000.0;
        let old = self.program.take();
        self.pending = None;

        if let (Some(old), true) = (&old, dip) {
            // Dip through black: old fades out over half, new fades in over half.
            let half = cue.transition.duration_ms as f64 / 2000.0;
            self.ramps.retain(|r| r.deck != old.deck);
            self.ramps.push(Ramp { deck: old.deck, from: self.fade_level[old.deck], to: 0.0, t0: self.now, dur: half, after: After::Clear });
            self.pending = Some(Pending { deck, cue_id: cue_id.to_string(), fade_in: half.max(fade_in) });
            self.info(format!("GO {} {} (dissolve {:.2}s)", cue.number, cue.name, half * 2.0));
            // The new cue becomes "program" once the old has faded.
            self.program = None;
            let _ = rate;
        } else {
            if let Some(old) = old {
                self.ramps.retain(|r| r.deck != old.deck);
                self.decks[old.deck].set_visible(false);
                self.decks[old.deck].unload();
                self.loaded[old.deck] = None;
            }
            self.start_on_air(deck, cue_id, fade_in);
            self.info(format!("GO {} {}", cue.number, cue.name));
        }

        // Advance standby to the following cue.
        let next = self.next_cue_after(cue_id);
        self.standby = next;
        self.preload();
    }

    fn start_on_air(&mut self, deck: usize, cue_id: &str, fade_in: f64) {
        let Some(cue) = self.cue(cue_id).cloned() else { return };
        let is_still = self.spec_for(&cue).map(|s| s.is_still).unwrap_or(false);
        let infinite_loop = cue.end == EndAction::Loop && cue.loop_count == 0;
        self.decks[deck].set_loop(infinite_loop);
        if fade_in > 0.0 {
            self.fade_level[deck] = 0.0;
            self.decks[deck].set_fade(0.0, 0.0);
            self.ramps.push(Ramp { deck, from: 0.0, to: 1.0, t0: self.now, dur: fade_in, after: After::Nothing });
        } else {
            self.fade_level[deck] = 1.0;
            self.decks[deck].set_fade(1.0, 1.0);
        }
        self.decks[deck].set_visible(!self.black);
        let phase = if cue.start == StartAction::PauseAtStart {
            Phase::Cued
        } else if is_still && cue.still_frames.is_none() {
            Phase::Hold
        } else {
            self.decks[deck].play();
            Phase::Playing
        };
        self.program = Some(OnAir {
            deck,
            cue_id: cue_id.to_string(),
            phase,
            loops_done: 0,
            looping_override: None,
            fade_out_started: false,
        });
    }

    fn clear_program(&mut self) {
        if let Some(p) = self.program.take() {
            self.decks[p.deck].set_visible(false);
            self.decks[p.deck].unload();
            self.loaded[p.deck] = None;
            self.ramps.retain(|r| r.deck != p.deck);
        }
    }

    fn fade_program_out(&mut self, seconds: f64) {
        let Some(p) = self.program.as_mut() else { return };
        p.phase = Phase::FadingOut;
        let d = p.deck;
        let from = self.fade_level[d];
        self.ramps.retain(|r| r.deck != d);
        self.ramps.push(Ramp { deck: d, from, to: 0.0, t0: self.now, dur: seconds.max(0.0), after: After::Clear });
    }

    pub fn handle(&mut self, cmd: Command) -> Result<(), String> {
        if self.locked && cmd.is_edit() {
            return Err("Show is locked".into());
        }
        match cmd {
            Command::Go => {
                if let Some(p) = &self.program {
                    if p.phase == Phase::Cued {
                        // Second GO releases a cue parked at its first frame.
                        let d = p.deck;
                        self.decks[d].play();
                        self.program.as_mut().unwrap().phase = Phase::Playing;
                        self.info("GO (release cued)");
                        return Ok(());
                    }
                }
                match self.standby.clone() {
                    Some(id) => self.take(&id),
                    None => return Err("No cue in standby".into()),
                }
            }
            Command::Take { cue_id } => self.take(&cue_id),
            Command::TakeNumber { number } => {
                let id = self.show.cue_by_number(&number).map(|i| self.show.cues[i].id.clone()).ok_or("unknown cue number")?;
                self.take(&id)
            }
            Command::Standby { cue_id } => {
                if self.cue(&cue_id).is_none() {
                    return Err("unknown cue".into());
                }
                self.set_standby(Some(cue_id))
            }
            Command::StandbyNumber { number } => {
                let id = self.show.cue_by_number(&number).map(|i| self.show.cues[i].id.clone()).ok_or("unknown cue number")?;
                self.set_standby(Some(id))
            }
            Command::StandbyNext | Command::StandbyPrev => {
                let n = self.show.cues.len();
                if n == 0 {
                    return Ok(());
                }
                let cur = self.standby.as_deref().and_then(|id| self.show.cue_index(id));
                let idx = match (cmd, cur) {
                    (Command::StandbyNext, Some(i)) => (i + 1).min(n - 1),
                    (Command::StandbyPrev, Some(i)) => i.saturating_sub(1),
                    (_, None) => 0,
                    _ => unreachable!(),
                };
                let id = self.show.cues[idx].id.clone();
                self.set_standby(Some(id))
            }
            Command::Pause => self.pause(),
            Command::Resume => self.resume(),
            Command::TogglePause => match self.program.as_ref().map(|p| p.phase) {
                Some(Phase::Playing) => self.pause(),
                Some(Phase::Paused) | Some(Phase::Cued) => self.resume(),
                _ => {}
            },
            Command::Stop => {
                self.ramps.clear();
                if let Some(p) = self.pending.take() {
                    self.decks[p.deck].unload();
                    self.loaded[p.deck] = None;
                }
                for d in 0..2 {
                    if self.program_deck() != Some(d) && self.loaded[d].is_some() && self.loaded[d] != self.standby {
                        self.decks[d].unload();
                        self.loaded[d] = None;
                    }
                }
                self.clear_program();
                self.info("STOP");
                self.preload();
            }
            Command::Panic => {
                if let Some(p) = self.pending.take() {
                    self.decks[p.deck].unload();
                    self.loaded[p.deck] = None;
                }
                self.fade_program_out(PANIC_SECONDS);
                self.info("PANIC");
            }
            Command::GotoRemain { seconds } => self.goto_remain(seconds)?,
            Command::Scrub { ratio } => self.scrub(ratio)?,
            Command::ToggleLoop => {
                if let Some(p) = self.program.as_mut() {
                    let cue_loop = self.show.cues.iter().find(|c| c.id == p.cue_id).map(|c| c.end == EndAction::Loop).unwrap_or(false);
                    let now = !p.looping_override.unwrap_or(cue_loop);
                    p.looping_override = Some(now);
                    let d = p.deck;
                    self.decks[d].set_loop(now);
                }
            }
            Command::Black { on } => self.set_black(on),
            Command::ToggleBlack => self.set_black(!self.black),
            Command::Bank { slot } => {
                let id = self.show.bank.iter().find(|b| b.slot.eq_ignore_ascii_case(&slot)).map(|b| b.cue_id.clone());
                match id {
                    Some(id) => {
                        let keep = self.standby.clone();
                        self.take(&id);
                        // Bank shots must not disturb the running order.
                        self.set_standby(keep);
                    }
                    None => return Err(format!("bank slot {slot} is empty")),
                }
            }
            Command::Lock { on } => {
                self.locked = on;
                self.info(if on { "show locked" } else { "show unlocked" });
            }
            Command::AddCue { cue, after } => {
                let idx = after.and_then(|a| self.show.cue_index(&a)).map(|i| i + 1).unwrap_or(self.show.cues.len());
                let first = self.show.cues.is_empty();
                self.show.cues.insert(idx, cue.clone());
                if first || self.standby.is_none() {
                    self.set_standby(Some(cue.id));
                }
                self.bump();
            }
            Command::UpdateCue { cue } => {
                let i = self.show.cue_index(&cue.id).ok_or("unknown cue")?;
                self.show.cues[i] = cue.clone();
                // Re-apply to any deck holding it.
                if let Some(spec) = self.spec_for(&cue) {
                    for d in 0..2 {
                        if self.loaded[d].as_deref() == Some(&cue.id) {
                            if self.busy(d) {
                                self.decks[d].apply(&spec);
                            } else {
                                // Parked: reload so IN point changes take effect.
                                self.load_into(d, &cue.id);
                            }
                        }
                    }
                }
                self.bump();
            }
            Command::DeleteCue { cue_id } => {
                if self.program.as_ref().map(|p| p.cue_id == cue_id).unwrap_or(false) {
                    return Err("cannot delete the cue on air".into());
                }
                let i = self.show.cue_index(&cue_id).ok_or("unknown cue")?;
                self.show.cues.remove(i);
                self.show.bank.retain(|b| b.cue_id != cue_id);
                if self.standby.as_deref() == Some(&cue_id) {
                    let next = self.show.cues.get(i).or_else(|| self.show.cues.last()).map(|c| c.id.clone());
                    self.set_standby(next);
                }
                self.bump();
            }
            Command::MoveCue { cue_id, to_index } => {
                let i = self.show.cue_index(&cue_id).ok_or("unknown cue")?;
                let c = self.show.cues.remove(i);
                let to = to_index.min(self.show.cues.len());
                self.show.cues.insert(to, c);
                self.bump();
            }
            Command::SetBank { slot, cue_id } => {
                self.show.bank.retain(|b| !b.slot.eq_ignore_ascii_case(&slot));
                if let Some(id) = cue_id {
                    self.show.bank.push(BankSlot { slot, cue_id: id });
                }
                self.bump();
            }
            Command::UpdateShow { mut settings } => {
                settings.output = settings.output.sanitized();
                self.show.show = settings;
                self.bump();
            }
            Command::UpsertMedia { media } => {
                match self.show.media.iter_mut().find(|m| m.id == media.id) {
                    Some(m) => *m = media,
                    None => self.show.media.push(media),
                }
                self.bump();
                self.preload();
            }
            Command::LoadShow { show } => {
                self.handle(Command::Stop)?;
                for d in 0..2 {
                    self.decks[d].unload();
                    self.loaded[d] = None;
                }
                self.standby = show.cues.first().map(|c| c.id.clone());
                self.show = *show;
                self.bump();
                self.preload();
            }
        }
        Ok(())
    }

    fn bump(&mut self) {
        self.show_rev += 1;
    }

    fn pause(&mut self) {
        if let Some(p) = self.program.as_mut() {
            if p.phase == Phase::Playing {
                p.phase = Phase::Paused;
                let d = p.deck;
                self.decks[d].pause();
            }
        }
    }
    fn resume(&mut self) {
        if let Some(p) = self.program.as_mut() {
            if matches!(p.phase, Phase::Paused | Phase::Cued) {
                p.phase = Phase::Playing;
                let d = p.deck;
                self.decks[d].play();
            }
        }
    }

    fn set_black(&mut self, on: bool) {
        self.black = on;
        if let Some(d) = self.program_deck() {
            self.decks[d].set_visible(!on);
        }
        self.info(if on { "output BLACK" } else { "output restored" });
    }

    fn on_air_bounds(&self) -> Option<(usize, f64, f64)> {
        let p = self.program.as_ref()?;
        let cue = self.cue(&p.cue_id)?;
        let rate = self.rate();
        let start = rate.frames_to_seconds(cue.in_frame);
        let end = rate.frames_to_seconds(self.show.cue_out(cue)?);
        Some((p.deck, start, end))
    }

    fn goto_remain(&mut self, seconds: f64) -> Result<(), String> {
        let (d, start, end) = self.on_air_bounds().ok_or("nothing with a known duration on air")?;
        let t = (end - seconds).max(start);
        self.decks[d].seek(t);
        if let Some(p) = self.program.as_mut() {
            p.fade_out_started = false;
        }
        self.info(format!("jump to -{seconds:.0}s"));
        Ok(())
    }

    fn scrub(&mut self, ratio: f64) -> Result<(), String> {
        if self.locked {
            return Err("Show is locked".into());
        }
        let (d, start, end) = self.on_air_bounds().ok_or("nothing with a known duration on air")?;
        self.decks[d].seek(start + (end - start) * ratio.clamp(0.0, 1.0));
        if let Some(p) = self.program.as_mut() {
            p.fade_out_started = false;
        }
        Ok(())
    }

    /// Advance timers and react to deck state. `now` is monotonic seconds.
    pub fn tick(&mut self, now: f64) {
        self.now = now;

        // 1) Ramps
        let mut done = vec![];
        for (i, r) in self.ramps.iter().enumerate() {
            let k = if r.dur <= 0.0 { 1.0 } else { ((now - r.t0) / r.dur).clamp(0.0, 1.0) };
            let v = r.from + (r.to - r.from) * k;
            self.fade_level[r.deck] = v;
            if k >= 1.0 {
                done.push(i);
            }
        }
        for d in 0..2 {
            if self.loaded[d].is_some() {
                let v = self.fade_level[d];
                self.decks[d].set_fade(v, v);
            }
        }
        for i in done.into_iter().rev() {
            let r = self.ramps.remove(i);
            match r.after {
                After::Nothing => {}
                After::Clear => {
                    if self.program_deck() == Some(r.deck) {
                        self.program = None;
                    }
                    self.decks[r.deck].set_visible(false);
                    self.decks[r.deck].unload();
                    self.loaded[r.deck] = None;
                    if self.pending.is_some() {
                        let p = self.pending.take().unwrap();
                        self.start_on_air(p.deck, &p.cue_id, p.fade_in);
                    }
                    self.preload();
                }
            }
        }
        // A dip transition whose outgoing side vanished (e.g. it ended) still has to start.
        if self.pending.is_some() && !self.ramps.iter().any(|r| r.after == After::Clear) {
            let p = self.pending.take().unwrap();
            self.start_on_air(p.deck, &p.cue_id, p.fade_in);
        }

        // 2) Program end handling
        let Some(p) = self.program.clone() else { return };
        let Some(cue) = self.cue(&p.cue_id).cloned() else { return };
        let st = self.decks[p.deck].status();
        let rate = self.rate();
        let out = self.show.cue_out(&cue);

        // Still with a hold time: emulate the clock.
        let is_still = self.spec_for(&cue).map(|s| s.is_still).unwrap_or(false);

        let fade_out = cue.audio.fade_out_ms as f64 / 1000.0;
        let looping = p.looping_override.unwrap_or(cue.end == EndAction::Loop);
        if p.phase == Phase::Playing && !is_still {
            if let Some(o) = out {
                let remain = rate.frames_to_seconds(o) - st.position;
                let ends_dark = matches!(cue.end, EndAction::Black | EndAction::Next | EndAction::Goto) && !looping;
                if ends_dark && fade_out > 0.0 && !p.fade_out_started && remain <= fade_out {
                    let d = p.deck;
                    let from = self.fade_level[d];
                    self.ramps.retain(|r| r.deck != d);
                    self.ramps.push(Ramp { deck: d, from, to: 0.0, t0: now, dur: remain.max(0.0), after: After::Nothing });
                    self.program.as_mut().unwrap().fade_out_started = true;
                }
            }
        }

        if p.phase == Phase::Playing && st.at_end {
            if looping && (p.looping_override == Some(true) || cue.loop_count == 0 || p.loops_done + 1 < cue.loop_count) {
                let start = rate.frames_to_seconds(cue.in_frame);
                self.decks[p.deck].seek(start);
                self.decks[p.deck].play();
                let pm = self.program.as_mut().unwrap();
                pm.loops_done += 1;
                pm.fade_out_started = false;
                return;
            }
            let end = if looping && cue.end == EndAction::Loop { EndAction::Black } else { cue.end };
            match end {
                EndAction::Hold => {
                    self.program.as_mut().unwrap().phase = Phase::Hold;
                }
                EndAction::Black | EndAction::Loop => {
                    self.clear_program();
                    self.preload();
                }
                EndAction::Next => match self.next_cue_after(&cue.id) {
                    Some(n) => self.take(&n),
                    None => {
                        self.clear_program();
                    }
                },
                EndAction::Goto => match cue.goto_cue_id.clone().filter(|g| self.cue(g).is_some()) {
                    Some(g) => self.take(&g),
                    None => {
                        self.warn(format!("cue {}: goto target missing", cue.number));
                        self.clear_program();
                    }
                },
            }
        }
    }

    pub fn state(&mut self) -> PlayoutState {
        let rate = self.rate();
        let mut dropped = 0;
        let mut level = (0.0f32, 0.0f32);
        for d in 0..2 {
            if self.loaded[d].is_some() {
                let st = self.decks[d].status();
                dropped += st.dropped_frames;
                if self.program_deck() == Some(d) {
                    level = st.level;
                }
            }
        }
        let program = self.program.clone().and_then(|p| {
            let cue = self.cue(&p.cue_id)?.clone();
            let st = self.decks[p.deck].status();
            let start = rate.frames_to_seconds(cue.in_frame);
            let elapsed_s = (st.position - start).max(0.0);
            let elapsed = rate.seconds_to_frame_floor(elapsed_s);
            let duration = self.show.cue_duration(&cue);
            let remain = duration.map(|d| d.saturating_sub(elapsed));
            Some(ProgramState {
                cue_id: cue.id.clone(),
                number: cue.number.clone(),
                name: cue.name.clone(),
                phase: p.phase,
                end: cue.end,
                looping: p.looping_override.unwrap_or(cue.end == EndAction::Loop),
                elapsed_frames: elapsed,
                remain_frames: remain,
                duration_frames: duration,
                progress: duration.filter(|d| *d > 0).map(|d| (elapsed as f64 / d as f64).min(1.0)),
                elapsed_tc: format_frames(elapsed, rate),
                remain_tc: remain.map(|r| format_frames(r, rate)).unwrap_or_else(|| "--:--:--.--".into()),
            })
        });
        let standby_ready = self
            .standby
            .as_ref()
            .map(|sb| (0..2).any(|d| self.loaded[d].as_ref() == Some(sb) && !self.busy(d)))
            .unwrap_or(false);
        PlayoutState {
            program,
            standby_cue_id: self.standby.clone(),
            standby_ready,
            black: self.black,
            locked: self.locked,
            dropped_frames: dropped,
            level,
            show_rev: self.show_rev,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::SimDeck;
    use crate::timecode::FrameRate;

    fn show() -> Show {
        let mut s = Show::default();
        s.show.frame_rate = FrameRate::Fps25;
        for (i, secs) in [10u64, 5, 8].iter().enumerate() {
            s.media.push(Media { id: format!("m{i}"), path: format!("clip{i}.mov"), duration_frames: secs * 25, ..Default::default() });
            s.cues.push(Cue { id: format!("c{i}"), ..Cue::new((i + 1).to_string(), format!("Cue {}", i + 1), Some(format!("m{i}"))) });
        }
        s
    }

    fn run(c: &mut Controller<SimDeck>, t: &mut f64, secs: f64) {
        let steps = (secs * 50.0) as usize;
        for _ in 0..steps {
            *t += 0.02;
            for d in c.decks.iter_mut() {
                d.advance(0.02);
            }
            c.tick(*t);
        }
    }

    #[test]
    fn go_plays_standby_and_preloads_next() {
        let mut c = Controller::new(show(), [SimDeck::default(), SimDeck::default()]);
        let st = c.state();
        assert_eq!(st.standby_cue_id.as_deref(), Some("c0"));
        assert!(st.standby_ready);
        c.handle(Command::Go).unwrap();
        let st = c.state();
        assert_eq!(st.program.as_ref().unwrap().cue_id, "c0");
        assert_eq!(st.standby_cue_id.as_deref(), Some("c1"));
        assert!(st.standby_ready, "next cue parked on the other deck");
        let d = c.program_deck().unwrap();
        assert!(c.decks[d].visible && c.decks[d].playing);
        assert!(!c.decks[1 - d].visible && !c.decks[1 - d].playing);
    }

    #[test]
    fn black_end_action_clears_and_remain_counts_down() {
        let mut c = Controller::new(show(), [SimDeck::default(), SimDeck::default()]);
        let mut t = 0.0;
        c.handle(Command::Go).unwrap();
        run(&mut c, &mut t, 4.0);
        let p = c.state().program.unwrap();
        assert_eq!(p.remain_tc, "00:00:06.00");
        run(&mut c, &mut t, 7.0);
        assert!(c.state().program.is_none(), "cue ended to black");
    }

    #[test]
    fn hold_next_and_loop() {
        let mut s = show();
        s.cues[0].end = EndAction::Next;
        s.cues[1].end = EndAction::Hold;
        let mut c = Controller::new(s, [SimDeck::default(), SimDeck::default()]);
        let mut t = 0.0;
        c.handle(Command::Go).unwrap();
        run(&mut c, &mut t, 10.5);
        let p = c.state().program.unwrap();
        assert_eq!(p.cue_id, "c1", "auto next");
        run(&mut c, &mut t, 6.0);
        let p = c.state().program.unwrap();
        assert_eq!(p.phase, Phase::Hold);

        let mut s = show();
        s.cues[1].end = EndAction::Loop;
        s.cues[1].loop_count = 2;
        let mut c = Controller::new(s, [SimDeck::default(), SimDeck::default()]);
        c.handle(Command::Take { cue_id: "c1".into() }).unwrap();
        let mut t = 0.0;
        run(&mut c, &mut t, 6.0);
        assert_eq!(c.state().program.unwrap().cue_id, "c1", "looping once");
        run(&mut c, &mut t, 5.0);
        assert!(c.state().program.is_none(), "after 2 passes ends black");
    }

    #[test]
    fn panic_fades_then_clears() {
        let mut c = Controller::new(show(), [SimDeck::default(), SimDeck::default()]);
        let mut t = 0.0;
        c.handle(Command::Go).unwrap();
        run(&mut c, &mut t, 1.0);
        c.handle(Command::Panic).unwrap();
        run(&mut c, &mut t, 0.5);
        let d = c.program_deck().unwrap();
        assert!(c.decks[d].fade.0 < 0.6 && c.decks[d].fade.0 > 0.4);
        run(&mut c, &mut t, 0.7);
        assert!(c.state().program.is_none());
    }

    #[test]
    fn dissolve_dips_through_black() {
        let mut s = show();
        s.cues[1].transition = Transition { kind: TransitionKind::Dissolve, duration_ms: 1000 };
        let mut c = Controller::new(s, [SimDeck::default(), SimDeck::default()]);
        let mut t = 0.0;
        c.handle(Command::Go).unwrap();
        run(&mut c, &mut t, 1.0);
        c.handle(Command::Go).unwrap();
        run(&mut c, &mut t, 0.25);
        assert!(c.state().program.is_none(), "outgoing fading");
        run(&mut c, &mut t, 0.4);
        let p = c.state().program.unwrap();
        assert_eq!(p.cue_id, "c1");
        run(&mut c, &mut t, 0.6);
        let d = c.program_deck().unwrap();
        assert!((c.decks[d].fade.0 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn goto_remain_and_lock() {
        let mut c = Controller::new(show(), [SimDeck::default(), SimDeck::default()]);
        let mut t = 0.0;
        c.handle(Command::Go).unwrap();
        c.handle(Command::GotoRemain { seconds: 3.0 }).unwrap();
        run(&mut c, &mut t, 0.02);
        assert_eq!(c.state().program.unwrap().remain_tc, "00:00:03.00");
        run(&mut c, &mut t, 0.04);
        assert_eq!(c.state().program.unwrap().remain_tc, "00:00:02.24");
        c.handle(Command::Lock { on: true }).unwrap();
        let cue = c.show.cues[0].clone();
        assert!(c.handle(Command::UpdateCue { cue }).is_err());
        assert!(c.handle(Command::StandbyNext).is_ok(), "operation still works when locked");
    }

    #[test]
    fn bank_does_not_move_standby() {
        let mut s = show();
        s.bank.push(BankSlot { slot: "F1".into(), cue_id: "c2".into() });
        let mut c = Controller::new(s, [SimDeck::default(), SimDeck::default()]);
        c.handle(Command::Bank { slot: "f1".into() }).unwrap();
        let st = c.state();
        assert_eq!(st.program.unwrap().cue_id, "c2");
        assert_eq!(st.standby_cue_id.as_deref(), Some("c0"));
    }

    #[test]
    fn pause_at_start_waits_for_second_go() {
        let mut s = show();
        s.cues[0].start = StartAction::PauseAtStart;
        let mut c = Controller::new(s, [SimDeck::default(), SimDeck::default()]);
        c.handle(Command::Go).unwrap();
        assert_eq!(c.state().program.unwrap().phase, Phase::Cued);
        c.handle(Command::Go).unwrap();
        let st = c.state();
        assert_eq!(st.program.unwrap().phase, Phase::Playing);
        assert_eq!(st.standby_cue_id.as_deref(), Some("c1"));
    }
}
