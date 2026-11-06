//! Show file model (`.trapdoor`, JSON). All times are stored in frames.

use serde::{Deserialize, Serialize};

use crate::timecode::FrameRate;

pub const FORMAT: &str = "trapdoor";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub format: String,
    pub version: u32,
    pub show: ShowSettings,
    #[serde(default)]
    pub media: Vec<Media>,
    #[serde(default)]
    pub cues: Vec<Cue>,
    #[serde(default)]
    pub bank: Vec<BankSlot>,
    #[serde(default)]
    pub remote: RemoteSettings,
}

impl Default for Show {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            version: FORMAT_VERSION,
            show: ShowSettings::default(),
            media: vec![],
            cues: vec![],
            bank: vec![],
            remote: RemoteSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShowSettings {
    pub name: String,
    pub frame_rate: FrameRate,
    pub output: OutputSettings,
    pub audio: AudioSettings,
    pub transition_default: Transition,
}

impl Default for ShowSettings {
    fn default() -> Self {
        Self {
            name: "Untitled Show".into(),
            frame_rate: FrameRate::Fps5994,
            output: OutputSettings::default(),
            audio: AudioSettings::default(),
            transition_default: Transition::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputSettings {
    /// 1-based display index. 0 = windowed preview (no fullscreen).
    pub display: u32,
    pub width: u32,
    pub height: u32,
    pub background: String,
    pub hide_cursor: bool,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self { display: 2, width: 1920, height: 1080, background: "#000000".into(), hide_cursor: true }
    }
}

/// Largest output raster supported (DCI 8K).
pub const MAX_OUTPUT: (u32, u32) = (8192, 4320);

/// Output presets offered in the UI.
pub const OUTPUT_PRESETS: &[(&str, u32, u32)] = &[
    ("HD 1080", 1920, 1080),
    ("WUXGA", 1920, 1200),
    ("UHD 4K", 3840, 2160),
    ("DCI 4K", 4096, 2160),
    ("UHD 8K", 7680, 4320),
    ("DCI 8K", 8192, 4320),
];

impl OutputSettings {
    /// Clamp to the supported range (keeps even sizes for chroma subsampling).
    pub fn sanitized(&self) -> Self {
        let mut o = self.clone();
        o.width = (o.width.clamp(320, MAX_OUTPUT.0)) & !1;
        o.height = (o.height.clamp(180, MAX_OUTPUT.1)) & !1;
        o
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    /// mpv audio-device name; "auto" = system default.
    pub device: String,
    pub master_db: f64,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { device: "auto".into(), master_db: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSettings {
    pub osc_port: u16,
    pub http_port: u16,
    pub enabled: bool,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self { osc_port: 53000, http_port: 8780, enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub rel_path: Option<String>,
    #[serde(default)]
    pub kind: MediaKind,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub fps: Option<f64>,
    /// Duration in show frames. Stills: 0 (duration comes from the cue).
    #[serde(default)]
    pub duration_frames: u64,
    #[serde(default)]
    pub audio_channels: u32,
    #[serde(default)]
    pub measured_lufs: Option<f64>,
    /// Waveform peaks, 0..=255, `PEAKS_PER_SECOND` per second (mono, max of channels).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peaks: Vec<u8>,
    /// Cached thumbnail strip (one frame per second), absolute path.
    #[serde(default)]
    pub filmstrip: Option<String>,
    #[serde(default)]
    pub thumb: Option<String>,
    #[serde(default)]
    pub analysis: AnalysisState,
}

pub const PEAKS_PER_SECOND: u32 = 100;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    #[default]
    Video,
    Still,
    Audio,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisState {
    #[default]
    Pending,
    Running,
    Done,
    Failed,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum StartAction {
    #[default]
    Play,
    PauseAtStart,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum EndAction {
    #[default]
    Black,
    Hold,
    Next,
    Loop,
    Goto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TransitionKind {
    #[default]
    Cut,
    Dissolve,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    #[serde(rename = "type")]
    pub kind: TransitionKind,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Fit {
    #[default]
    Fit,
    Fill,
    Stretch,
    Native,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRoute {
    /// 1-based source channel.
    pub src: u32,
    /// "L" | "R" | "1".."8"
    pub out: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Normalize {
    pub enabled: bool,
    pub target_lufs: f64,
}

impl Default for Normalize {
    fn default() -> Self {
        Self { enabled: false, target_lufs: -23.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioAdjust {
    pub gain_db: f64,
    pub mute: bool,
    /// -1.0 (L) .. 1.0 (R)
    pub pan: f64,
    pub normalize: Normalize,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    #[serde(default)]
    pub map: Vec<ChannelRoute>,
}

impl Default for AudioAdjust {
    fn default() -> Self {
        Self {
            gain_db: 0.0,
            mute: false,
            pan: 0.0,
            normalize: Normalize::default(),
            fade_in_ms: 0,
            fade_out_ms: 0,
            map: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Crop {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PictureAdjust {
    pub fit: Fit,
    /// 1.0 = 100 %
    pub scale: f64,
    pub x: i32,
    pub y: i32,
    pub crop: Crop,
    /// 0.0 .. 1.0 (rendered over black)
    pub opacity: f64,
    pub flip_h: bool,
    pub flip_v: bool,
    /// -100 .. 100
    pub brightness: i32,
    pub contrast: i32,
    pub saturation: i32,
}

impl Default for PictureAdjust {
    fn default() -> Self {
        Self {
            fit: Fit::Fit,
            scale: 1.0,
            x: 0,
            y: 0,
            crop: Crop::default(),
            opacity: 1.0,
            flip_h: false,
            flip_v: false,
            brightness: 0,
            contrast: 0,
            saturation: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: String,
    pub number: String,
    pub name: String,
    pub media_id: Option<String>,
    pub in_frame: u64,
    /// None = media end
    pub out_frame: Option<u64>,
    /// Stills only: hold time in frames (None = until next GO)
    #[serde(default)]
    pub still_frames: Option<u64>,
    pub start: StartAction,
    pub end: EndAction,
    /// 0 = infinite
    pub loop_count: u32,
    pub goto_cue_id: Option<String>,
    pub transition: Transition,
    pub speed: f64,
    pub audio: AudioAdjust,
    pub picture: PictureAdjust,
    pub color: Option<String>,
    pub note: String,
    pub hotkey: Option<String>,
}

impl Cue {
    pub fn new(number: impl Into<String>, name: impl Into<String>, media_id: Option<String>) -> Self {
        Self {
            id: format!("c_{}", uuid::Uuid::new_v4().simple()),
            number: number.into(),
            name: name.into(),
            media_id,
            in_frame: 0,
            out_frame: None,
            still_frames: None,
            start: StartAction::Play,
            end: EndAction::Black,
            loop_count: 0,
            goto_cue_id: None,
            transition: Transition::default(),
            speed: 1.0,
            audio: AudioAdjust::default(),
            picture: PictureAdjust::default(),
            color: None,
            note: String::new(),
            hotkey: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BankSlot {
    /// "F1".."F12", "S-F1".."S-F12"
    pub slot: String,
    pub cue_id: String,
}

impl Show {
    pub fn media(&self, id: &str) -> Option<&Media> {
        self.media.iter().find(|m| m.id == id)
    }
    pub fn cue_index(&self, id: &str) -> Option<usize> {
        self.cues.iter().position(|c| c.id == id)
    }
    pub fn cue_by_number(&self, number: &str) -> Option<usize> {
        self.cues.iter().position(|c| c.number == number)
    }
    /// Effective OUT frame (exclusive) for a cue, if known.
    pub fn cue_out(&self, cue: &Cue) -> Option<u64> {
        if let Some(o) = cue.out_frame {
            return Some(o);
        }
        let m = cue.media_id.as_deref().and_then(|id| self.media(id))?;
        match m.kind {
            MediaKind::Still => cue.still_frames.map(|f| cue.in_frame + f),
            _ if m.duration_frames > 0 => Some(m.duration_frames),
            _ => None,
        }
    }
    pub fn cue_duration(&self, cue: &Cue) -> Option<u64> {
        self.cue_out(cue).map(|o| o.saturating_sub(cue.in_frame))
    }
    /// Gain (dB) actually applied: user gain + loudness normalisation.
    pub fn applied_gain_db(&self, cue: &Cue) -> f64 {
        let mut g = cue.audio.gain_db + self.show.audio.master_db;
        if cue.audio.normalize.enabled {
            if let Some(l) = cue.media_id.as_deref().and_then(|id| self.media(id)).and_then(|m| m.measured_lufs) {
                g += cue.audio.normalize.target_lufs - l;
            }
        }
        g.clamp(-60.0, 12.0)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        let show: Show = serde_json::from_str(s)?;
        Ok(show)
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("show serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let mut show = Show::default();
        show.media.push(Media { id: "m1".into(), path: "a.mov".into(), duration_frames: 600, ..Default::default() });
        let mut c = Cue::new("1", "Opening", Some("m1".into()));
        c.end = EndAction::Hold;
        show.cues.push(c);
        let s = show.to_json();
        assert!(s.contains("\"format\": \"trapdoor\""));
        let back = Show::from_json(&s).unwrap();
        assert_eq!(back, show);
    }

    #[test]
    fn durations_and_gain() {
        let mut show = Show::default();
        show.media.push(Media { id: "m1".into(), duration_frames: 600, measured_lufs: Some(-18.0), ..Default::default() });
        let mut c = Cue::new("1", "x", Some("m1".into()));
        c.in_frame = 100;
        assert_eq!(show.cue_duration(&c), Some(500));
        c.out_frame = Some(300);
        assert_eq!(show.cue_duration(&c), Some(200));
        c.audio.gain_db = -3.0;
        c.audio.normalize.enabled = true;
        assert!((show.applied_gain_db(&c) - (-8.0)).abs() < 1e-9);
    }
}
