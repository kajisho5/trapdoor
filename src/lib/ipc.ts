// One interface for the operator UI: the desktop engine (Tauri) or the browser simulator.

import type { Command, DisplayInfo, EngineInfo, LogEntry, OutputSettings, PlayoutState, Preset, Show } from "../types";
import { SimEngine, demoShow } from "./sim";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Unlisten = () => void;

export interface Engine {
  command(cmd: Command): Promise<void>;
  getShow(): Promise<Show>;
  info(): Promise<EngineInfo>;
  presets(): Promise<Preset[]>;
  displays(): Promise<DisplayInfo[]>;
  applyOutput(o: OutputSettings): Promise<void>;
  addMedia(paths: string[], after: string | null): Promise<string[]>;
  pickMedia(): Promise<string[]>;
  saveShow(path?: string): Promise<string | null>;
  openShow(): Promise<boolean>;
  newShow(): Promise<void>;
  relink(mediaId: string): Promise<void>;
  onPlayout(f: (s: PlayoutState) => void): Promise<Unlisten>;
  onShow(f: (s: Show) => void): Promise<Unlisten>;
  onLog(f: (l: LogEntry) => void): Promise<Unlisten>;
  onDrop(f: (paths: string[]) => void): Promise<Unlisten>;
  fileUrl(path: string | null | undefined): string | null;
}

const VIDEO_EXT = ["mov", "mp4", "m4v", "mxf", "avi", "mkv", "webm", "png", "jpg", "jpeg", "tif", "tiff", "bmp", "wav", "mp3", "aac", "m4a", "flac", "aif", "aiff"];

async function tauriEngine(): Promise<Engine> {
  const core = await import("@tauri-apps/api/core");
  const ev = await import("@tauri-apps/api/event");
  const web = await import("@tauri-apps/api/webview");
  const dialog = await import("@tauri-apps/plugin-dialog");
  let currentPath: string | null = null;
  const inv = core.invoke;
  return {
    command: (cmd) => inv("engine_command", { cmd }),
    getShow: () => inv("get_show"),
    info: () => inv("engine_info"),
    presets: () => inv("output_presets"),
    displays: () => inv("list_displays"),
    applyOutput: (output) => inv("apply_output", { output }),
    addMedia: (paths, after) => inv("add_media", { paths, after }),
    pickMedia: async () => {
      const r = await dialog.open({ multiple: true, filters: [{ name: "Media", extensions: VIDEO_EXT }] });
      return r == null ? [] : Array.isArray(r) ? r : [r];
    },
    saveShow: async (path) => {
      let p = path ?? currentPath;
      if (!p) {
        p = await dialog.save({ filters: [{ name: "TRAPDOOR show", extensions: ["trapdoor"] }] });
        if (!p) return null;
        if (!p.toLowerCase().endsWith(".trapdoor")) p += ".trapdoor";
      }
      await inv("save_show", { path: p });
      currentPath = p;
      return p;
    },
    openShow: async () => {
      const p = await dialog.open({ multiple: false, filters: [{ name: "TRAPDOOR show", extensions: ["trapdoor", "autosave"] }] });
      if (!p || Array.isArray(p)) return false;
      await inv("open_show", { path: p });
      currentPath = p.endsWith(".autosave") ? null : p;
      return true;
    },
    newShow: async () => {
      await inv("new_show");
      currentPath = null;
    },
    relink: async (mediaId) => {
      const p = await dialog.open({ multiple: false, filters: [{ name: "Media", extensions: VIDEO_EXT }] });
      if (!p || Array.isArray(p)) return;
      await inv("relink_media", { mediaId, path: p });
    },
    onPlayout: (f) => ev.listen<PlayoutState>("playout", (e) => f(e.payload)),
    onShow: (f) => ev.listen<Show>("show", (e) => f(e.payload)),
    onLog: (f) => ev.listen<LogEntry>("engine-log", (e) => f(e.payload)),
    onDrop: (f) =>
      web.getCurrentWebview().onDragDropEvent((e) => {
        if (e.payload.type === "drop") f(e.payload.paths);
      }),
    fileUrl: (p) => (p ? (p.startsWith("demo:") ? p : core.convertFileSrc(p)) : null),
  };
}

function simEngine(): Engine {
  const sim = new SimEngine(demoShow());
  (window as unknown as { __trapdoorSim: SimEngine }).__trapdoorSim = sim;
  const ok = async <T>(v: T) => v;
  return {
    command: async (cmd) => sim.handle(cmd),
    getShow: () => ok(structuredClone(sim.show)),
    info: () => ok({ version: "0.1.0", mpvError: "browser simulator", ffmpeg: false, output: "SIMULATOR" }),
    presets: () =>
      ok([
        { label: "HD 1080", width: 1920, height: 1080 },
        { label: "WUXGA", width: 1920, height: 1200 },
        { label: "UHD 4K", width: 3840, height: 2160 },
        { label: "DCI 4K", width: 4096, height: 2160 },
        { label: "UHD 8K", width: 7680, height: 4320 },
        { label: "DCI 8K", width: 8192, height: 4320 },
      ]),
    displays: () => ok([{ index: 1, width: 1920, height: 1080, primary: true }, { index: 2, width: 1920, height: 1080, primary: false }]),
    applyOutput: async (o) => sim.handle({ cmd: "updateShow", settings: { ...sim.show.show, output: o } }),
    addMedia: () => ok([]),
    pickMedia: () => ok([]),
    saveShow: () => ok(null),
    openShow: () => ok(false),
    newShow: async () => {},
    relink: async () => {},
    onPlayout: async (f) => sim.onPlayout(f) as unknown as Unlisten,
    onShow: async (f) => sim.onShow(f) as unknown as Unlisten,
    onLog: async (f) => sim.onLog(f) as unknown as Unlisten,
    onDrop: async () => () => {},
    fileUrl: (p) => p ?? null,
  };
}

let enginePromise: Promise<Engine> | null = null;
export function engine(): Promise<Engine> {
  if (!enginePromise) enginePromise = isTauri ? tauriEngine() : Promise.resolve(simEngine());
  return enginePromise;
}
