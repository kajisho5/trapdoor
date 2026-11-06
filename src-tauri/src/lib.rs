//! TRAPDOOR desktop app: wires the engine (decks, controller, remote) to the operator UI.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use trapdoor_engine::analysis::{kind_for, Analyzer};
use trapdoor_engine::controller::{Command, Controller};
use trapdoor_engine::model::{AnalysisState, Cue, EndAction, Media, MediaKind, OutputSettings, Show, OUTPUT_PRESETS};
use trapdoor_engine::player::{ClockedSimDeck, Deck};
use trapdoor_engine::remote::{self, RemoteCtx};

type Reply<T> = mpsc::Sender<Result<T, String>>;

enum Msg {
    Cmd(Command, Option<Reply<()>>),
    GetShow(Reply<Show>),
    RebuildOutput(Reply<()>),
    SetPath(Option<PathBuf>),
}

struct AppState {
    tx: mpsc::Sender<Msg>,
    show_path: Mutex<Option<PathBuf>>,
    analyzer: Arc<Analyzer>,
    info: RwLock<EngineInfo>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct EngineInfo {
    version: String,
    /// None = libmpv loaded; Some(reason) = not available
    mpv_error: Option<String>,
    ffmpeg: bool,
    output: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PlayoutEvent<'a> {
    #[serde(flatten)]
    state: &'a trapdoor_engine::PlayoutState,
    standby_name: Option<String>,
    standby_number: Option<String>,
}

fn send<T>(st: &AppState, make: impl FnOnce(Reply<T>) -> Msg) -> Result<T, String> {
    let (tx, rx) = mpsc::channel();
    st.tx.send(make(tx)).map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(10)).map_err(|e| e.to_string())?
}

// ---------------- decks ----------------

struct Output {
    #[cfg(windows)]
    win: Option<trapdoor_engine::output_win::OutputWindow>,
}

impl Drop for Output {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(w) = &self.win {
            w.close();
        }
    }
}

fn make_decks(out: &OutputSettings) -> ([Box<dyn Deck>; 2], Output, String) {
    if let Err(e) = trapdoor_engine::mpv::available() {
        let d: [Box<dyn Deck>; 2] = [Box::new(ClockedSimDeck::default()), Box::new(ClockedSimDeck::default())];
        return (d, Output { #[cfg(windows)] win: None }, format!("SIMULATION (libmpv not found: {e})"));
    }
    #[cfg(windows)]
    {
        use trapdoor_engine::mpv::MpvDeck;
        use trapdoor_engine::output_win::OutputWindow;
        match OutputWindow::spawn(out.display, (out.width, out.height)) {
            Ok(win) => {
                let mk = |child: isize| -> Result<Box<dyn Deck>, String> {
                    let d = MpvDeck::new(Some(child as i64), Box::new(move |v| OutputWindow::set_child_visible(child, v)))?;
                    Ok(Box::new(d))
                };
                match (mk(win.children[0]), mk(win.children[1])) {
                    (Ok(a), Ok(b)) => {
                        let label = format!("DISP{} {}x{}", out.display, out.width, out.height);
                        return ([a, b], Output { win: Some(win) }, label);
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        win.close();
                        let d: [Box<dyn Deck>; 2] = [Box::new(ClockedSimDeck::default()), Box::new(ClockedSimDeck::default())];
                        return (d, Output { win: None }, format!("SIMULATION (mpv: {e})"));
                    }
                }
            }
            Err(e) => {
                let d: [Box<dyn Deck>; 2] = [Box::new(ClockedSimDeck::default()), Box::new(ClockedSimDeck::default())];
                return (d, Output { win: None }, format!("SIMULATION (output window: {e})"));
            }
        }
    }
    #[cfg(not(windows))]
    {
        use trapdoor_engine::mpv::MpvDeck;
        // Development on macOS/Linux: each deck opens its own mpv window.
        let mk = || -> Result<Box<dyn Deck>, String> { Ok(Box::new(MpvDeck::new(None, Box::new(|_| {}))?)) };
        match (mk(), mk()) {
            (Ok(a), Ok(b)) => ([a, b], Output {}, format!("DEV WINDOWS {}x{}", out.width, out.height)),
            (Err(e), _) | (_, Err(e)) => {
                let d: [Box<dyn Deck>; 2] = [Box::new(ClockedSimDeck::default()), Box::new(ClockedSimDeck::default())];
                (d, Output {}, format!("SIMULATION (mpv: {e})"))
            }
        }
    }
}

// ---------------- engine thread ----------------

fn autosave_path(app: &AppHandle, show_path: &Option<PathBuf>) -> PathBuf {
    match show_path {
        Some(p) => PathBuf::from(format!("{}.autosave", p.display())),
        None => app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir()).join("untitled.trapdoor.autosave"),
    }
}

fn engine_thread(app: AppHandle, rx: mpsc::Receiver<Msg>, show: Show, remote_state: Arc<RwLock<serde_json::Value>>, mut remote_rx: tokio::sync::mpsc::UnboundedReceiver<Command>) {
    let (decks, mut output, label) = make_decks(&show.show.output);
    app.state::<AppState>().info.write().output = label;
    let mut ctl = Controller::new(show, decks);
    let t0 = Instant::now();
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    let mut last_rev = 0;
    let mut dirty_since: Option<Instant> = None;
    let mut show_path: Option<PathBuf> = None;
    let mut log_seen = 0usize;
    loop {
        // Commands from the UI
        loop {
            match rx.try_recv() {
                Ok(Msg::Cmd(c, reply)) => {
                    let r = ctl.handle(c);
                    if let Some(tx) = reply {
                        let _ = tx.send(r);
                    }
                }
                Ok(Msg::GetShow(tx)) => {
                    let _ = tx.send(Ok(ctl.show.clone()));
                }
                Ok(Msg::RebuildOutput(tx)) => {
                    let show = ctl.show.clone();
                    let locked = ctl.locked();
                    drop(ctl);
                    drop(output);
                    let (decks, o, label) = make_decks(&show.show.output);
                    output = o;
                    app.state::<AppState>().info.write().output = label;
                    ctl = Controller::new(show, decks);
                    let _ = ctl.handle(Command::Lock { on: locked });
                    last_rev = 0;
                    log_seen = 0;
                    let _ = tx.send(Ok(()));
                }
                Ok(Msg::SetPath(p)) => show_path = p,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }
        // Commands from OSC / HTTP
        while let Ok(c) = remote_rx.try_recv() {
            if c.is_edit() {
                continue; // remote control never edits the show
            }
            if let Err(e) = ctl.handle(c) {
                let _ = app.emit("engine-log", serde_json::json!({"level": "warn", "msg": format!("remote: {e}")}));
            }
        }

        ctl.tick(t0.elapsed().as_secs_f64());

        if last_emit.elapsed() >= Duration::from_millis(33) {
            last_emit = Instant::now();
            let st = ctl.state();
            let sb = st.standby_cue_id.as_deref().and_then(|id| ctl.show.cues.iter().find(|c| c.id == id));
            let ev = PlayoutEvent { state: &st, standby_name: sb.map(|c| c.name.clone()), standby_number: sb.map(|c| c.number.clone()) };
            let json = serde_json::to_value(&ev).unwrap_or_default();
            *remote_state.write() = json.clone();
            let _ = app.emit("playout", json);
            if st.show_rev != last_rev {
                if last_rev != 0 {
                    dirty_since = Some(Instant::now());
                }
                last_rev = st.show_rev;
                let _ = app.emit("show", &ctl.show);
            }
            let total = ctl.log.len();
            if total < log_seen {
                log_seen = 0;
            }
            for e in ctl.log.iter().skip(log_seen) {
                let _ = app.emit("engine-log", e);
            }
            log_seen = total;
        }
        if let Some(t) = dirty_since {
            if t.elapsed() > Duration::from_secs(2) {
                dirty_since = None;
                let _ = std::fs::write(autosave_path(&app, &show_path), ctl.show.to_json());
            }
        }
        thread::sleep(Duration::from_millis(4));
    }
}

// ---------------- commands ----------------

#[tauri::command]
fn engine_command(st: State<AppState>, cmd: Command) -> Result<(), String> {
    send(&st, |r| Msg::Cmd(cmd, Some(r)))
}

#[tauri::command]
fn get_show(st: State<AppState>) -> Result<Show, String> {
    send(&st, Msg::GetShow)
}

#[tauri::command]
fn engine_info(st: State<AppState>) -> EngineInfo {
    st.info.read().clone()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Preset {
    label: &'static str,
    width: u32,
    height: u32,
}

#[tauri::command]
fn output_presets() -> Vec<Preset> {
    OUTPUT_PRESETS.iter().map(|(l, w, h)| Preset { label: l, width: *w, height: *h }).collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Display {
    index: u32,
    width: i32,
    height: i32,
    primary: bool,
}

#[tauri::command]
fn list_displays() -> Vec<Display> {
    #[cfg(windows)]
    {
        trapdoor_engine::output_win::displays()
            .into_iter()
            .map(|d| Display { index: d.index, width: d.width, height: d.height, primary: d.primary })
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![Display { index: 1, width: 1920, height: 1080, primary: true }]
    }
}

#[tauri::command]
fn apply_output(st: State<AppState>, output: OutputSettings) -> Result<(), String> {
    let mut show = send(&st, Msg::GetShow)?;
    show.show.output = output.sanitized();
    send(&st, |r| Msg::Cmd(Command::UpdateShow { settings: show.show.clone() }, Some(r)))?;
    send(&st, Msg::RebuildOutput)
}

fn rel_to(path: &Path, base: &Path) -> Option<String> {
    path.strip_prefix(base).ok().map(|p| p.to_string_lossy().replace('\\', "/"))
}

fn spawn_analysis(app: &AppHandle, media: Media) {
    let app = app.clone();
    thread::spawn(move || {
        let st = app.state::<AppState>();
        let rate = send(&st, Msg::GetShow).map(|s| s.show.frame_rate).unwrap_or_default();
        let mut m = media;
        m.analysis = AnalysisState::Running;
        let _ = st.tx.send(Msg::Cmd(Command::UpsertMedia { media: m.clone() }, None));
        let done = st.analyzer.analyze(&m, rate);
        let _ = st.tx.send(Msg::Cmd(Command::UpsertMedia { media: done }, None));
    });
}

/// Add media files as new cues (after `after`, or at the end).
#[tauri::command]
fn add_media(app: AppHandle, st: State<AppState>, paths: Vec<String>, after: Option<String>) -> Result<Vec<String>, String> {
    let show = send(&st, Msg::GetShow)?;
    let base = st.show_path.lock().as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let mut next_num = show.cues.iter().filter_map(|c| c.number.parse::<u32>().ok()).max().unwrap_or(0) + 1;
    let mut after = after;
    let mut ids = vec![];
    for p in paths {
        let path = PathBuf::from(&p);
        let media = Media {
            id: format!("m_{}", &uuid_like()),
            path: p.clone(),
            rel_path: base.as_deref().and_then(|b| rel_to(&path, b)),
            kind: kind_for(&p),
            ..Default::default()
        };
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Untitled".into());
        let mut cue = Cue::new(next_num.to_string(), name, Some(media.id.clone()));
        if media.kind == MediaKind::Still {
            cue.end = EndAction::Hold;
        }
        next_num += 1;
        send(&st, |r| Msg::Cmd(Command::UpsertMedia { media: media.clone() }, Some(r)))?;
        send(&st, |r| Msg::Cmd(Command::AddCue { cue: cue.clone(), after: after.clone() }, Some(r)))?;
        after = Some(cue.id.clone());
        ids.push(cue.id);
        spawn_analysis(&app, media);
    }
    Ok(ids)
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:x}{:04x}", t, n & 0xffff)
}

#[tauri::command]
fn save_show(st: State<AppState>, path: String) -> Result<(), String> {
    let mut show = send(&st, Msg::GetShow)?;
    let p = PathBuf::from(&path);
    if let Some(dir) = p.parent() {
        for m in show.media.iter_mut() {
            m.rel_path = rel_to(Path::new(&m.path), dir).or(m.rel_path.take());
        }
    }
    std::fs::write(&p, show.to_json()).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(format!("{path}.autosave"));
    *st.show_path.lock() = Some(p.clone());
    let _ = st.tx.send(Msg::SetPath(Some(p)));
    Ok(())
}

#[tauri::command]
fn open_show(app: AppHandle, st: State<AppState>, path: String) -> Result<(), String> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut show = Show::from_json(&text).map_err(|e| format!("not a TRAPDOOR show: {e}"))?;
    let dir = Path::new(&path).parent().map(|d| d.to_path_buf());
    // Relink: absolute path first, then relative to the show file.
    for m in show.media.iter_mut() {
        if !Path::new(&m.path).exists() {
            if let (Some(d), Some(rel)) = (&dir, &m.rel_path) {
                let cand = d.join(rel);
                if cand.exists() {
                    m.path = cand.to_string_lossy().into_owned();
                }
            }
        }
        if !Path::new(&m.path).exists() {
            m.analysis = AnalysisState::Missing;
        }
    }
    let needs: Vec<Media> = show.media.iter().filter(|m| m.analysis != AnalysisState::Done && m.analysis != AnalysisState::Missing).cloned().collect();
    send(&st, |r| Msg::Cmd(Command::LoadShow { show: Box::new(show) }, Some(r)))?;
    let p = PathBuf::from(&path);
    *st.show_path.lock() = Some(p.clone());
    let _ = st.tx.send(Msg::SetPath(Some(p)));
    for m in needs {
        spawn_analysis(&app, m);
    }
    Ok(())
}

#[tauri::command]
fn new_show(st: State<AppState>) -> Result<(), String> {
    let mut show = Show::default();
    if let Ok(cur) = send(&st, Msg::GetShow) {
        show.show.output = cur.show.output;
        show.show.audio = cur.show.audio;
    }
    send(&st, |r| Msg::Cmd(Command::LoadShow { show: Box::new(show) }, Some(r)))?;
    *st.show_path.lock() = None;
    let _ = st.tx.send(Msg::SetPath(None));
    Ok(())
}

#[tauri::command]
fn relink_media(app: AppHandle, st: State<AppState>, media_id: String, path: String) -> Result<(), String> {
    let show = send(&st, Msg::GetShow)?;
    let mut m = show.media.iter().find(|m| m.id == media_id).cloned().ok_or("unknown media")?;
    m.path = path;
    m.analysis = AnalysisState::Pending;
    send(&st, |r| Msg::Cmd(Command::UpsertMedia { media: m.clone() }, Some(r)))?;
    spawn_analysis(&app, m);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let cache = app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir()).join("media");
            let (tx, rx) = mpsc::channel();
            let ffmpeg_ok = std::process::Command::new(trapdoor_engine::analysis::tool("ffprobe")).arg("-version").output().map(|o| o.status.success()).unwrap_or(false);
            app.manage(AppState {
                tx,
                show_path: Mutex::new(None),
                analyzer: Arc::new(Analyzer::new(cache)),
                info: RwLock::new(EngineInfo {
                    version: env!("CARGO_PKG_VERSION").into(),
                    mpv_error: trapdoor_engine::mpv::available().err(),
                    ffmpeg: ffmpeg_ok,
                    output: String::new(),
                }),
            });
            let show = Show::default();
            let remote_settings = show.remote.clone();
            let remote_state = Arc::new(RwLock::new(serde_json::Value::Null));
            let (rtx, rrx) = tokio::sync::mpsc::unbounded_channel();
            let ctx = RemoteCtx { tx: rtx, state: remote_state.clone() };
            if remote_settings.enabled {
                let h2 = handle.clone();
                thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("tokio");
                    rt.block_on(remote::serve(ctx, remote_settings.osc_port, remote_settings.http_port, move |e| {
                        let _ = h2.emit("engine-log", serde_json::json!({"level": "warn", "msg": e}));
                    }));
                });
            }
            thread::Builder::new().name("trapdoor-engine".into()).spawn(move || engine_thread(handle, rx, show, remote_state, rrx))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_command,
            get_show,
            engine_info,
            output_presets,
            list_displays,
            apply_output,
            add_media,
            save_show,
            open_show,
            new_show,
            relink_media
        ])
        .run(tauri::generate_context!())
        .expect("error while running TRAPDOOR");
}
