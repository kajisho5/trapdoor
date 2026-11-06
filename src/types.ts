// Mirrors crates/engine/src/model.rs and controller.rs (serde camelCase).

export type FrameRate = "23.976" | "24" | "25" | "29.97" | "30" | "50" | "59.94" | "60";
export type MediaKind = "video" | "still" | "audio";
export type AnalysisState = "pending" | "running" | "done" | "failed" | "missing";
export type StartAction = "play" | "pauseAtStart";
export type EndAction = "black" | "hold" | "next" | "loop" | "goto";
export type TransitionKind = "cut" | "dissolve";
export type Fit = "fit" | "fill" | "stretch" | "native";
export type Phase = "playing" | "paused" | "hold" | "cued" | "fadingOut";

export interface OutputSettings {
  display: number;
  width: number;
  height: number;
  background: string;
  hideCursor: boolean;
}

export interface ShowSettings {
  name: string;
  frameRate: FrameRate;
  output: OutputSettings;
  audio: { device: string; masterDb: number };
  transitionDefault: Transition;
}

export interface Transition {
  type: TransitionKind;
  durationMs: number;
}

export interface Media {
  id: string;
  path: string;
  relPath?: string | null;
  kind: MediaKind;
  codec?: string | null;
  width: number;
  height: number;
  fps?: number | null;
  durationFrames: number;
  audioChannels: number;
  measuredLufs?: number | null;
  peaks?: number[];
  filmstrip?: string | null;
  thumb?: string | null;
  analysis: AnalysisState;
}

export interface ChannelRoute {
  src: number;
  out: string;
}

export interface AudioAdjust {
  gainDb: number;
  mute: boolean;
  pan: number;
  normalize: { enabled: boolean; targetLufs: number };
  fadeInMs: number;
  fadeOutMs: number;
  map: ChannelRoute[];
}

export interface PictureAdjust {
  fit: Fit;
  scale: number;
  x: number;
  y: number;
  crop: { top: number; bottom: number; left: number; right: number };
  opacity: number;
  flipH: boolean;
  flipV: boolean;
  brightness: number;
  contrast: number;
  saturation: number;
}

export interface Cue {
  id: string;
  number: string;
  name: string;
  mediaId: string | null;
  inFrame: number;
  outFrame: number | null;
  stillFrames?: number | null;
  start: StartAction;
  end: EndAction;
  loopCount: number;
  gotoCueId: string | null;
  transition: Transition;
  speed: number;
  audio: AudioAdjust;
  picture: PictureAdjust;
  color: string | null;
  note: string;
  hotkey: string | null;
}

export interface BankSlot {
  slot: string;
  cueId: string;
}

export interface Show {
  format: string;
  version: number;
  show: ShowSettings;
  media: Media[];
  cues: Cue[];
  bank: BankSlot[];
  remote: { oscPort: number; httpPort: number; enabled: boolean };
}

export interface ProgramState {
  cueId: string;
  number: string;
  name: string;
  phase: Phase;
  end: EndAction;
  looping: boolean;
  elapsedFrames: number;
  remainFrames: number | null;
  durationFrames: number | null;
  progress: number | null;
  elapsedTc: string;
  remainTc: string;
}

export interface PlayoutState {
  program: ProgramState | null;
  standbyCueId: string | null;
  standbyReady: boolean;
  black: boolean;
  locked: boolean;
  droppedFrames: number;
  level: [number, number];
  showRev: number;
  standbyName?: string | null;
  standbyNumber?: string | null;
}

export interface LogEntry {
  t?: number;
  level: string;
  msg: string;
}

export interface EngineInfo {
  version: string;
  mpvError: string | null;
  ffmpeg: boolean;
  output: string;
}

export type Command =
  | { cmd: "go" }
  | { cmd: "take"; cueId: string }
  | { cmd: "takeNumber"; number: string }
  | { cmd: "standby"; cueId: string }
  | { cmd: "standbyNext" }
  | { cmd: "standbyPrev" }
  | { cmd: "pause" }
  | { cmd: "resume" }
  | { cmd: "togglePause" }
  | { cmd: "stop" }
  | { cmd: "panic" }
  | { cmd: "gotoRemain"; seconds: number }
  | { cmd: "scrub"; ratio: number }
  | { cmd: "toggleLoop" }
  | { cmd: "black"; on: boolean }
  | { cmd: "toggleBlack" }
  | { cmd: "bank"; slot: string }
  | { cmd: "lock"; on: boolean }
  | { cmd: "addCue"; cue: Cue; after: string | null }
  | { cmd: "updateCue"; cue: Cue }
  | { cmd: "deleteCue"; cueId: string }
  | { cmd: "moveCue"; cueId: string; toIndex: number }
  | { cmd: "setBank"; slot: string; cueId: string | null }
  | { cmd: "updateShow"; settings: ShowSettings };

export interface Preset {
  label: string;
  width: number;
  height: number;
}

export interface DisplayInfo {
  index: number;
  width: number;
  height: number;
  primary: boolean;
}
