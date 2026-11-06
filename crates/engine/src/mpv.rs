//! libmpv deck. libmpv is loaded at runtime with `libloading`, so the app builds
//! without mpv headers/import libraries and ships `libmpv-2.dll` next to the exe.
//!
//! Each deck is one mpv instance rendering into a window handle (`wid`) given by the
//! output window. Showing/hiding the deck is done by the output window (child windows),
//! so a cut is instant: the incoming deck is already decoded and parked on its first frame.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::{Arc, OnceLock};

use crate::model::Fit;
use crate::player::{Deck, DeckStatus, LoadSpec};

type Handle = *mut c_void;

#[allow(non_snake_case)]
pub struct Api {
    _lib: libloading::Library,
    create: unsafe extern "C" fn() -> Handle,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    terminate_destroy: unsafe extern "C" fn(Handle),
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    get_property_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    command: unsafe extern "C" fn(Handle, *mut *const c_char) -> c_int,
    free: unsafe extern "C" fn(*mut c_void),
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

static API: OnceLock<Result<Arc<Api>, String>> = OnceLock::new();

fn lib_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["libmpv-2.dll", "mpv-2.dll", "mpv-1.dll"]
    } else if cfg!(target_os = "macos") {
        &["libmpv.2.dylib", "libmpv.dylib"]
    } else {
        &["libmpv.so.2", "libmpv.so"]
    }
}

fn load_api() -> Result<Arc<Api>, String> {
    let mut last = String::new();
    // Look next to the executable first, then the normal search path.
    let mut candidates: Vec<std::path::PathBuf> = vec![];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for n in lib_names() {
                candidates.push(dir.join(n));
            }
        }
    }
    for n in lib_names() {
        candidates.push(n.into());
    }
    for path in candidates {
        match unsafe { libloading::Library::new(&path) } {
            Ok(lib) => unsafe {
                macro_rules! sym {
                    ($name:literal) => {
                        *lib.get(concat!($name, "\0").as_bytes()).map_err(|e| format!("{}: {e}", $name))?
                    };
                }
                let api = Api {
                    create: sym!("mpv_create"),
                    initialize: sym!("mpv_initialize"),
                    terminate_destroy: sym!("mpv_terminate_destroy"),
                    set_option_string: sym!("mpv_set_option_string"),
                    set_property_string: sym!("mpv_set_property_string"),
                    get_property_string: sym!("mpv_get_property_string"),
                    command: sym!("mpv_command"),
                    free: sym!("mpv_free"),
                    error_string: sym!("mpv_error_string"),
                    _lib: lib,
                };
                return Ok(Arc::new(api));
            },
            Err(e) => last = format!("{}: {e}", path.display()),
        }
    }
    Err(format!("libmpv not found ({last})"))
}

pub fn api() -> Result<Arc<Api>, String> {
    API.get_or_init(load_api).clone()
}

/// Is libmpv available on this machine?
pub fn available() -> Result<(), String> {
    api().map(|_| ())
}

pub struct MpvDeck {
    api: Arc<Api>,
    h: Handle,
    spec: Option<LoadSpec>,
    fade: (f64, f64),
    visible_cb: Box<dyn FnMut(bool) + Send>,
    looping: bool,
}

unsafe impl Send for MpvDeck {}

fn cs(s: &str) -> CString {
    CString::new(s).unwrap_or_default()
}

impl MpvDeck {
    /// `wid`: native window handle to render into (HWND on Windows). `None` = mpv's own window.
    /// `on_visible`: called to show/hide this deck's window.
    pub fn new(wid: Option<i64>, on_visible: Box<dyn FnMut(bool) + Send>) -> Result<Self, String> {
        let api = api()?;
        let h = unsafe { (api.create)() };
        if h.is_null() {
            return Err("mpv_create failed".into());
        }
        let d = Self { api, h, spec: None, fade: (1.0, 1.0), visible_cb: on_visible, looping: false };
        let mut opts: Vec<(&str, String)> = vec![
            ("vo", "gpu-next".into()),
            ("hwdec", "auto-safe".into()),
            ("keep-open", "always".into()),
            ("idle", "yes".into()),
            ("force-window", "yes".into()),
            ("osc", "no".into()),
            ("osd-level", "0".into()),
            ("input-default-bindings", "no".into()),
            ("input-vo-keyboard", "no".into()),
            ("cursor-autohide", "always".into()),
            ("background", "color".into()),
            ("background-color", "#000000".into()),
            ("terminal", "no".into()),
            ("audio-display", "no".into()),
            ("image-display-duration", "inf".into()),
            ("hr-seek", "yes".into()),
            ("hr-seek-framedrop", "no".into()),
            ("video-sync", "display-resample".into()),
            ("interpolation", "no".into()),
            ("volume-max", "400".into()),
            ("demuxer-max-bytes", "512MiB".into()),
            ("cache", "yes".into()),
            ("pause", "yes".into()),
        ];
        if let Some(w) = wid {
            opts.push(("wid", w.to_string()));
        }
        for (k, v) in &opts {
            unsafe {
                (d.api.set_option_string)(d.h, cs(k).as_ptr(), cs(v).as_ptr());
            }
        }
        let r = unsafe { (d.api.initialize)(d.h) };
        if r < 0 {
            return Err(format!("mpv_initialize: {}", d.err(r)));
        }
        Ok(d)
    }

    fn err(&self, code: c_int) -> String {
        unsafe { CStr::from_ptr((self.api.error_string)(code)).to_string_lossy().into_owned() }
    }

    fn set(&self, k: &str, v: &str) {
        unsafe {
            (self.api.set_property_string)(self.h, cs(k).as_ptr(), cs(v).as_ptr());
        }
    }

    fn get(&self, k: &str) -> Option<String> {
        unsafe {
            let p = (self.api.get_property_string)(self.h, cs(k).as_ptr());
            if p.is_null() {
                return None;
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            (self.api.free)(p as *mut c_void);
            Some(s)
        }
    }

    fn cmd(&self, args: &[&str]) -> Result<(), String> {
        let owned: Vec<CString> = args.iter().map(|a| cs(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        let r = unsafe { (self.api.command)(self.h, ptrs.as_mut_ptr()) };
        if r < 0 {
            Err(self.err(r))
        } else {
            Ok(())
        }
    }

    fn apply_picture_audio(&self, spec: &LoadSpec) {
        let p = &spec.picture;
        // Fit
        match p.fit {
            Fit::Fit => {
                self.set("keepaspect", "yes");
                self.set("panscan", "0");
                self.set("video-unscaled", "no");
            }
            Fit::Fill => {
                self.set("keepaspect", "yes");
                self.set("panscan", "1");
                self.set("video-unscaled", "no");
            }
            Fit::Stretch => {
                self.set("keepaspect", "no");
                self.set("video-unscaled", "no");
            }
            Fit::Native => {
                self.set("keepaspect", "yes");
                self.set("video-unscaled", "yes");
            }
        }
        self.set("video-zoom", &format!("{:.4}", p.scale.max(0.05).log2()));
        // mpv pan is relative to the scaled video size; approximate pixels on a 1920 wide raster.
        self.set("video-pan-x", &format!("{:.5}", p.x as f64 / 1920.0));
        self.set("video-pan-y", &format!("{:.5}", p.y as f64 / 1080.0));
        let c = p.crop;
        if c.top + c.bottom + c.left + c.right > 0 {
            if let (Some(w), Some(h)) = (self.get("width").and_then(|s| s.parse::<u32>().ok()), self.get("height").and_then(|s| s.parse::<u32>().ok())) {
                let cw = w.saturating_sub(c.left + c.right).max(2);
                let ch = h.saturating_sub(c.top + c.bottom).max(2);
                self.set("video-crop", &format!("{cw}x{ch}+{}+{}", c.left, c.top));
            }
        } else {
            self.set("video-crop", "");
        }
        let mut vf = vec![];
        if p.flip_h {
            vf.push("hflip");
        }
        if p.flip_v {
            vf.push("vflip");
        }
        self.set("vf", &vf.join(","));
        self.set("contrast", &p.contrast.clamp(-100, 100).to_string());
        self.set("saturation", &p.saturation.clamp(-100, 100).to_string());
        self.set("speed", &format!("{:.3}", spec.speed));
        // Audio routing: channel map + pan → lavfi pan filter
        let af = audio_filter(spec);
        self.set("af", &af);
        if spec.audio_device != "auto" && !spec.audio_device.is_empty() {
            self.set("audio-device", &spec.audio_device);
        }
        self.push_fade();
    }

    fn push_fade(&self) {
        let Some(spec) = &self.spec else { return };
        let (pic, aud) = self.fade;
        // Picture: opacity and fades are rendered over black by darkening.
        let o = (spec.picture.opacity * pic).clamp(0.0, 1.0);
        let user = spec.picture.brightness as f64;
        let b = user + (-100.0 - user) * (1.0 - o);
        self.set("brightness", &format!("{:.1}", b.clamp(-100.0, 100.0)));
        let mute = spec.audio.mute || aud <= 0.0;
        self.set("mute", if mute { "yes" } else { "no" });
        let lin = 10f64.powf(spec.gain_db / 20.0) * aud;
        self.set("volume", &format!("{:.2}", (lin * 100.0).clamp(0.0, 400.0)));
    }
}

/// Build an mpv `af` string for channel routing and pan.
pub fn audio_filter(spec: &LoadSpec) -> String {
    let a = &spec.audio;
    let mut parts: Vec<String> = vec![];
    if !a.map.is_empty() {
        let mut l: Vec<String> = vec![];
        let mut r: Vec<String> = vec![];
        for m in &a.map {
            let src = format!("c{}", m.src.saturating_sub(1));
            match m.out.as_str() {
                "L" | "1" => l.push(src),
                "R" | "2" => r.push(src),
                _ => {}
            }
        }
        if !l.is_empty() || !r.is_empty() {
            let join = |v: &Vec<String>| if v.is_empty() { "0*c0".to_string() } else { v.join("+") };
            parts.push(format!("lavfi=[pan=stereo|c0={}|c1={}]", join(&l), join(&r)));
        }
    }
    if a.pan.abs() > 0.001 {
        let p = a.pan.clamp(-1.0, 1.0);
        let lg = if p > 0.0 { 1.0 - p } else { 1.0 };
        let rg = if p < 0.0 { 1.0 + p } else { 1.0 };
        parts.push(format!("lavfi=[pan=stereo|c0={lg:.3}*c0|c1={rg:.3}*c1]"));
    }
    parts.join(",")
}

impl Deck for MpvDeck {
    fn load(&mut self, spec: &LoadSpec) -> Result<(), String> {
        self.spec = Some(spec.clone());
        self.fade = (1.0, 1.0);
        self.set("pause", "yes");
        self.set("loop-file", "no");
        // Per-file options so they never leak into the next load.
        let end = spec.end.map(|e| format!("{e:.6}")).unwrap_or_else(|| "none".into());
        let opts = format!("start={:.6},end={end}", spec.start);
        self.cmd(&["loadfile", &spec.path, "replace", "-1", &opts])?;
        self.apply_picture_audio(spec);
        Ok(())
    }
    fn unload(&mut self) {
        let _ = self.cmd(&["stop"]);
        self.spec = None;
    }
    fn play(&mut self) {
        self.set("pause", "no");
    }
    fn pause(&mut self) {
        self.set("pause", "yes");
    }
    fn seek(&mut self, seconds: f64) {
        let _ = self.cmd(&["seek", &format!("{seconds:.6}"), "absolute+exact"]);
    }
    fn set_fade(&mut self, picture: f64, audio: f64) {
        if (self.fade.0 - picture).abs() > 1e-4 || (self.fade.1 - audio).abs() > 1e-4 {
            self.fade = (picture, audio);
            self.push_fade();
        }
    }
    fn set_visible(&mut self, visible: bool) {
        (self.visible_cb)(visible);
    }
    fn set_loop(&mut self, enabled: bool) {
        self.looping = enabled;
        // With start/end set, loop-file loops the IN..OUT range (mpv ab-loop semantics).
        if let Some(spec) = &self.spec {
            if enabled {
                self.set("ab-loop-a", &format!("{:.6}", spec.start));
                match spec.end {
                    Some(e) => self.set("ab-loop-b", &format!("{e:.6}")),
                    None => self.set("ab-loop-b", "no"),
                }
                self.set("loop-file", "inf");
            } else {
                self.set("ab-loop-a", "no");
                self.set("ab-loop-b", "no");
                self.set("loop-file", "no");
            }
        }
    }
    fn apply(&mut self, spec: &LoadSpec) {
        self.spec = Some(spec.clone());
        self.apply_picture_audio(spec);
    }
    fn status(&mut self) -> DeckStatus {
        let loaded = self.spec.is_some() && self.get("path").is_some();
        let position = self.get("time-pos").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let paused = self.get("pause").as_deref() == Some("yes");
        let eof = self.get("eof-reached").as_deref() == Some("yes");
        let past_end = match self.spec.as_ref().and_then(|s| s.end) {
            Some(e) => position >= e - 0.5 / 60.0,
            None => false,
        };
        let dropped = self.get("frame-drop-count").and_then(|s| s.parse().ok()).unwrap_or(0)
            + self.get("vo-delayed-frame-count").and_then(|s| s.parse().ok()).unwrap_or(0);
        DeckStatus {
            position,
            loaded,
            paused,
            at_end: loaded && !self.looping && (eof || past_end),
            dropped_frames: dropped,
            level: if !paused && loaded { (0.0, 0.0) } else { (0.0, 0.0) },
        }
    }
}

impl Drop for MpvDeck {
    fn drop(&mut self) {
        unsafe { (self.api.terminate_destroy)(self.h) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AudioAdjust, ChannelRoute, PictureAdjust};

    fn spec(a: AudioAdjust) -> LoadSpec {
        LoadSpec {
            path: "x".into(),
            start: 0.0,
            end: None,
            is_still: false,
            speed: 1.0,
            audio: a,
            gain_db: 0.0,
            picture: PictureAdjust::default(),
            audio_device: "auto".into(),
        }
    }

    #[test]
    fn filters() {
        assert_eq!(audio_filter(&spec(AudioAdjust::default())), "");
        let mut a = AudioAdjust::default();
        a.map = vec![ChannelRoute { src: 3, out: "L".into() }, ChannelRoute { src: 4, out: "R".into() }];
        assert_eq!(audio_filter(&spec(a.clone())), "lavfi=[pan=stereo|c0=c2|c1=c3]");
        a.map.clear();
        a.pan = 0.5;
        assert_eq!(audio_filter(&spec(a)), "lavfi=[pan=stereo|c0=0.500*c0|c1=1.000*c1]");
    }
}
