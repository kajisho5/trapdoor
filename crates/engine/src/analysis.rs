//! Media analysis through the ffprobe / ffmpeg command-line tools (shipped next to the exe).
//! Runs in the background when media is added, never during playback decisions.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::model::{AnalysisState, Media, MediaKind, PEAKS_PER_SECOND};
use crate::timecode::FrameRate;

const STILL_EXT: &[&str] = &["png", "jpg", "jpeg", "tif", "tiff", "bmp", "webp"];
const AUDIO_EXT: &[&str] = &["wav", "mp3", "aac", "m4a", "flac", "aif", "aiff", "ogg"];

pub fn tool(name: &str) -> PathBuf {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    if let Ok(me) = std::env::current_exe() {
        if let Some(dir) = me.parent() {
            let p = dir.join(&exe);
            if p.exists() {
                return p;
            }
        }
    }
    PathBuf::from(exe)
}

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

pub fn kind_for(path: &str) -> MediaKind {
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if STILL_EXT.contains(&ext.as_str()) {
        MediaKind::Still
    } else if AUDIO_EXT.contains(&ext.as_str()) {
        MediaKind::Audio
    } else {
        MediaKind::Video
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Probe {
    pub codec: Option<String>,
    pub width: u32,
    pub height: u32,
    pub fps: Option<f64>,
    pub duration: f64,
    pub audio_channels: u32,
}

/// Parse `ffprobe -print_format json -show_streams -show_format` output.
pub fn parse_probe(json: &str) -> Result<Probe, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut p = Probe::default();
    if let Some(streams) = v["streams"].as_array() {
        for s in streams {
            match s["codec_type"].as_str() {
                Some("video") if p.codec.is_none() && s["disposition"]["attached_pic"].as_i64() != Some(1) => {
                    p.codec = s["codec_name"].as_str().map(|c| {
                        let prof = s["profile"].as_str().unwrap_or("");
                        if c == "prores" && !prof.is_empty() { format!("ProRes {prof}") } else { c.to_string() }
                    });
                    p.width = s["width"].as_u64().unwrap_or(0) as u32;
                    p.height = s["height"].as_u64().unwrap_or(0) as u32;
                    p.fps = s["avg_frame_rate"].as_str().or(s["r_frame_rate"].as_str()).and_then(parse_ratio);
                    if let Some(d) = s["duration"].as_str().and_then(|d| d.parse::<f64>().ok()) {
                        p.duration = p.duration.max(d);
                    }
                }
                Some("audio") => {
                    p.audio_channels = p.audio_channels.max(s["channels"].as_u64().unwrap_or(0) as u32);
                }
                _ => {}
            }
        }
    }
    if let Some(d) = v["format"]["duration"].as_str().and_then(|d| d.parse::<f64>().ok()) {
        if p.duration <= 0.0 {
            p.duration = d;
        }
    }
    Ok(p)
}

fn parse_ratio(s: &str) -> Option<f64> {
    let (a, b) = s.split_once('/')?;
    let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
    if b == 0.0 || a == 0.0 { None } else { Some(a / b) }
}

/// Convert signed 16-bit mono samples (any rate) to 0..255 peaks at `PEAKS_PER_SECOND`.
pub fn peaks_from_s16(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let per = (sample_rate / PEAKS_PER_SECOND).max(1) as usize;
    samples
        .chunks(per)
        .map(|c| {
            let m = c.iter().map(|s| (*s as i32).unsigned_abs()).max().unwrap_or(0);
            ((m as f64 / 32768.0).sqrt() * 255.0).round().min(255.0) as u8
        })
        .collect()
}

/// Parse the integrated loudness ("I: -18.4 LUFS") from ffmpeg's ebur128 summary.
pub fn parse_ebur128(stderr: &str) -> Option<f64> {
    let summary = stderr.rsplit("Summary:").next()?;
    for line in summary.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("I:") {
            let n = rest.trim().split_whitespace().next()?;
            return n.parse().ok();
        }
    }
    None
}

pub struct Analyzer {
    pub cache_dir: PathBuf,
}

impl Analyzer {
    pub fn new(cache_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&cache_dir);
        Self { cache_dir }
    }

    pub fn probe(&self, path: &str) -> Result<Probe, String> {
        let out = hidden(&mut Command::new(tool("ffprobe")))
            .args(["-v", "error", "-print_format", "json", "-show_streams", "-show_format", path])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("ffprobe: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        parse_probe(&String::from_utf8_lossy(&out.stdout))
    }

    fn key(&self, media: &Media) -> String {
        let meta = std::fs::metadata(&media.path).ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut h: u64 = 0xcbf29ce484222325;
        for b in media.path.bytes().chain(size.to_le_bytes()).chain(mtime.to_le_bytes()) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        format!("{h:016x}")
    }

    /// Full analysis. Returns an updated copy of `media`.
    pub fn analyze(&self, media: &Media, rate: FrameRate) -> Media {
        let mut m = media.clone();
        if !Path::new(&m.path).exists() {
            m.analysis = AnalysisState::Missing;
            return m;
        }
        m.kind = kind_for(&m.path);
        let probe = match self.probe(&m.path) {
            Ok(p) => p,
            Err(_) => {
                m.analysis = AnalysisState::Failed;
                return m;
            }
        };
        m.codec = probe.codec.clone();
        m.width = probe.width;
        m.height = probe.height;
        m.fps = probe.fps;
        m.audio_channels = probe.audio_channels;
        m.duration_frames = if m.kind == MediaKind::Still { 0 } else { rate.seconds_to_frames(probe.duration) };

        let key = self.key(&m);
        let thumb = self.cache_dir.join(format!("{key}_thumb.jpg"));
        let strip = self.cache_dir.join(format!("{key}_strip.jpg"));
        if m.kind != MediaKind::Audio {
            let at = if m.kind == MediaKind::Still { 0.0 } else { (probe.duration * 0.1).min(3.0) };
            let ok = hidden(&mut Command::new(tool("ffmpeg")))
                .args(["-v", "error", "-y", "-ss", &format!("{at:.3}"), "-i", &m.path, "-frames:v", "1", "-vf", "scale=320:-2", "-q:v", "4"])
                .arg(&thumb)
                .stdin(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                m.thumb = Some(thumb.to_string_lossy().into_owned());
            }
            if m.kind == MediaKind::Video && probe.duration > 0.0 {
                // One frame per second, tiled horizontally, 160 px wide each (max 600 tiles).
                let n = (probe.duration.ceil() as u32).clamp(1, 600);
                let ok = hidden(&mut Command::new(tool("ffmpeg")))
                    .args(["-v", "error", "-y", "-i", &m.path, "-vf"])
                    .arg(format!("fps=1,scale=160:90:force_original_aspect_ratio=decrease,pad=160:90:(ow-iw)/2:(oh-ih)/2,tile={n}x1"))
                    .args(["-frames:v", "1", "-q:v", "5"])
                    .arg(&strip)
                    .stdin(Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ok {
                    m.filmstrip = Some(strip.to_string_lossy().into_owned());
                }
            }
        }
        if probe.audio_channels > 0 {
            // Waveform peaks
            let sr = 8000u32;
            if let Ok(out) = hidden(&mut Command::new(tool("ffmpeg")))
                .args(["-v", "error", "-i", &m.path, "-vn", "-ac", "1", "-ar", &sr.to_string(), "-f", "s16le", "-"])
                .stdin(Stdio::null())
                .output()
            {
                let samples: Vec<i16> = out.stdout.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
                m.peaks = peaks_from_s16(&samples, sr);
            }
            // Integrated loudness
            if let Ok(out) = hidden(&mut Command::new(tool("ffmpeg")))
                .args(["-hide_banner", "-nostats", "-i", &m.path, "-vn", "-af", "ebur128", "-f", "null", "-"])
                .stdin(Stdio::null())
                .output()
            {
                m.measured_lufs = parse_ebur128(&String::from_utf8_lossy(&out.stderr));
            }
        }
        m.analysis = AnalysisState::Done;
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_json() {
        let j = r#"{"streams":[{"codec_type":"video","codec_name":"prores","profile":"HQ","width":1920,"height":1080,"avg_frame_rate":"60000/1001","duration":"20.02"},{"codec_type":"audio","channels":2}],"format":{"duration":"20.05"}}"#;
        let p = parse_probe(j).unwrap();
        assert_eq!(p.codec.as_deref(), Some("ProRes HQ"));
        assert_eq!((p.width, p.height, p.audio_channels), (1920, 1080, 2));
        assert!((p.fps.unwrap() - 59.94).abs() < 0.01);
        assert!((p.duration - 20.02).abs() < 1e-9);
    }

    #[test]
    fn loudness_and_peaks() {
        let s = "  ...\n[Parsed_ebur128_0 @ 0x1] Summary:\n\n  Integrated loudness:\n    I:         -18.4 LUFS\n    Threshold: -28.6 LUFS\n";
        assert_eq!(parse_ebur128(s), Some(-18.4));
        let samples: Vec<i16> = (0..8000).map(|i| if i < 4000 { 0 } else { 16384 }).collect();
        let p = peaks_from_s16(&samples, 8000);
        assert_eq!(p.len(), 100);
        assert_eq!(p[0], 0);
        assert!(p[99] > 170);
        assert_eq!(kind_for("A.PNG"), MediaKind::Still);
        assert_eq!(kind_for("x.wav"), MediaKind::Audio);
    }

    /// Runs only when ffmpeg is installed (CI / dev machines).
    #[test]
    fn analyze_real_file_if_ffmpeg_present() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("t.mov");
        let ok = Command::new("ffmpeg")
            .args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=25:duration=3", "-f", "lavfi", "-i", "sine=frequency=440:duration=3", "-shortest", "-c:v", "mjpeg", "-c:a", "pcm_s16le"])
            .arg(&f)
            .status()
            .unwrap()
            .success();
        assert!(ok);
        let a = Analyzer::new(dir.path().join("cache"));
        let m = Media { id: "m".into(), path: f.to_string_lossy().into(), ..Default::default() };
        let r = a.analyze(&m, FrameRate::Fps25);
        assert_eq!(r.analysis, AnalysisState::Done);
        assert_eq!(r.duration_frames, 75);
        assert_eq!(r.audio_channels, 1);
        assert!(r.thumb.is_some() && r.filmstrip.is_some());
        assert!((r.peaks.len() as i64 - 300).abs() <= 2, "{}", r.peaks.len());
        assert!(r.measured_lufs.is_some());
    }
}
