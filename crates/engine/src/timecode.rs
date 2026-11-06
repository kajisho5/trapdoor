//! Frame rates and timecode formatting.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FrameRate {
    #[serde(rename = "23.976")]
    Fps23976,
    #[serde(rename = "24")]
    Fps24,
    #[serde(rename = "25")]
    Fps25,
    #[serde(rename = "29.97")]
    Fps2997,
    #[serde(rename = "30")]
    Fps30,
    #[serde(rename = "50")]
    Fps50,
    #[default]
    #[serde(rename = "59.94")]
    Fps5994,
    #[serde(rename = "60")]
    Fps60,
}

impl FrameRate {
    pub fn num_den(self) -> (u64, u64) {
        match self {
            FrameRate::Fps23976 => (24000, 1001),
            FrameRate::Fps24 => (24, 1),
            FrameRate::Fps25 => (25, 1),
            FrameRate::Fps2997 => (30000, 1001),
            FrameRate::Fps30 => (30, 1),
            FrameRate::Fps50 => (50, 1),
            FrameRate::Fps5994 => (60000, 1001),
            FrameRate::Fps60 => (60, 1),
        }
    }
    pub fn fps(self) -> f64 {
        let (n, d) = self.num_den();
        n as f64 / d as f64
    }
    /// Nominal frames per "timecode second" (rounded rate), used for display.
    pub fn nominal(self) -> u64 {
        self.fps().round() as u64
    }
    /// Durations: nearest frame.
    pub fn seconds_to_frames(self, s: f64) -> u64 {
        if s <= 0.0 {
            return 0;
        }
        (s * self.fps()).round() as u64
    }
    /// Playheads: the frame currently showing.
    pub fn seconds_to_frame_floor(self, s: f64) -> u64 {
        if s <= 0.0 {
            return 0;
        }
        (s * self.fps() + 1e-6).floor() as u64
    }
    pub fn frames_to_seconds(self, f: u64) -> f64 {
        let (n, d) = self.num_den();
        f as f64 * d as f64 / n as f64
    }
    pub fn ms_to_frames(self, ms: u32) -> u64 {
        self.seconds_to_frames(ms as f64 / 1000.0)
    }
    pub fn from_fps(fps: f64) -> Option<Self> {
        let all = [
            FrameRate::Fps23976,
            FrameRate::Fps24,
            FrameRate::Fps25,
            FrameRate::Fps2997,
            FrameRate::Fps30,
            FrameRate::Fps50,
            FrameRate::Fps5994,
            FrameRate::Fps60,
        ];
        all.into_iter().find(|r| (r.fps() - fps).abs() < 0.01)
    }
}

/// `HH:MM:SS.FF`: real elapsed time shown on the nominal frame grid, so 59.94 material
/// reads like a wall clock (90 s of 59.94 shows 00:01:30.00).
pub fn format_frames(frames: u64, rate: FrameRate) -> String {
    let nominal = rate.nominal().max(1);
    let n = (rate.frames_to_seconds(frames) * nominal as f64).round() as u64;
    let whole = n / nominal;
    let ff = n % nominal;
    format!("{:02}:{:02}:{:02}.{:02}", whole / 3600, (whole / 60) % 60, whole % 60, ff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        let r = FrameRate::Fps25;
        assert_eq!(format_frames(0, r), "00:00:00.00");
        assert_eq!(format_frames(25 * 61 + 3, r), "00:01:01.03");
        let r = FrameRate::Fps5994;
        assert_eq!(format_frames(r.seconds_to_frames(90.0), r), "00:01:30.00");
        assert_eq!(format_frames(r.seconds_to_frames(30.0), r), "00:00:30.00");
        assert_eq!(r.seconds_to_frames(1.0), 60);
        assert_eq!(r.seconds_to_frame_floor(1.0), 59);
    }

    #[test]
    fn serde_names() {
        assert_eq!(serde_json::to_string(&FrameRate::Fps5994).unwrap(), "\"59.94\"");
        assert_eq!(FrameRate::from_fps(29.97), Some(FrameRate::Fps2997));
    }
}
