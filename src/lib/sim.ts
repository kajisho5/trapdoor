// Browser-only simulated engine. Lets the operator UI run without the desktop app
// (design review, rehearsal, screenshots). Logic mirrors crates/engine/src/controller.rs
// in a simplified form. All sample content is fictional.

import type { Command, Cue, LogEntry, Media, PlayoutState, ProgramState, Show } from "../types";
import { formatFrames, fps, framesToSeconds, secondsToFrames } from "./tc";

type Listener<T> = (v: T) => void;

function fakePeaks(seconds: number, seed: number): number[] {
  const n = Math.round(seconds * 100);
  const out: number[] = [];
  for (let i = 0; i < n; i++) {
    const t = i / Math.max(n - 1, 1);
    const env = 0.35 + 0.45 * Math.abs(Math.sin(t * 3.1 * seed + seed)) + 0.2 * Math.abs(Math.sin(t * 23 + seed * 2));
    const v = env * (0.75 + 0.25 * Math.abs(Math.sin(i * 1.7 + seed * 5) * Math.cos(i * 0.43 + seed)));
    out.push(Math.min(255, Math.round(v * 230)));
  }
  return out;
}

export function newCue(number: string, name: string, mediaId: string | null): Cue {
  return {
    id: `c_${Math.random().toString(36).slice(2, 10)}`,
    number,
    name,
    mediaId,
    inFrame: 0,
    outFrame: null,
    stillFrames: null,
    start: "play",
    end: "black",
    loopCount: 0,
    gotoCueId: null,
    transition: { type: "cut", durationMs: 0 },
    speed: 1,
    audio: { gainDb: 0, mute: false, pan: 0, normalize: { enabled: false, targetLufs: -23 }, fadeInMs: 0, fadeOutMs: 0, map: [] },
    picture: { fit: "fit", scale: 1, x: 0, y: 0, crop: { top: 0, bottom: 0, left: 0, right: 0 }, opacity: 1, flipH: false, flipV: false, brightness: 0, contrast: 0, saturation: 0 },
    color: null,
    note: "",
    hotkey: null,
  };
}

export function demoShow(): Show {
  const rate = "59.94" as const;
  const items: [string, string, string, number, Cue["end"], number, string, string, Media["kind"]][] = [
    ["1", "Pre-show Loop", "Runs during house open", 30, "loop", -6, "logo", "Sample_Loop_A.mov", "video"],
    ["2", "Countdown 10s", "GO on MC 'starting soon'", 10, "next", 0, "count", "Sample_Countdown.mov", "video"],
    ["3", "Opening Video", "", 90, "hold", 0, "open", "Sample_Opening.mov", "video"],
    ["4", "Guest Intro BG Loop", "Name supers on separate system", 20, "loop", -12, "guest", "guest_loop.mov", "video"],
    ["5", "Awards BGM", "Audio only, picture black", 165.2, "black", -3, "bgm", "Sample_BGM.wav", "audio"],
    ["6", "Award Winner Photos", "8 stills, 5s each", 40, "next", 0, "award", "Sample_Photos.mov", "video"],
    ["7", "Intermission Loop", "", 45, "loop", -9, "break", "Sample_Intermission.mov", "video"],
    ["8", "Sponsor Reel", "", 60, "goto", -6, "sponsor", "Sample_Sponsor.mov", "video"],
    ["9", "Closing Credits", "GO when applause settles", 130.08, "black", 0, "end", "Sample_Closing.mov", "video"],
  ];
  const media: Media[] = [];
  const cues: Cue[] = [];
  items.forEach(([num, name, note, secs, end, gain, look, file, kind], i) => {
    const id = `m_demo${i}`;
    media.push({
      id,
      path: `D:/SampleShow/media/${file}`,
      relPath: `media/${file}`,
      kind,
      codec: kind === "audio" ? "pcm_s24le" : "ProRes 422 HQ",
      width: kind === "audio" ? 0 : 1920,
      height: kind === "audio" ? 0 : 1080,
      fps: kind === "audio" ? null : 59.94,
      durationFrames: secondsToFrames(secs, rate),
      audioChannels: look === "award" ? 0 : 2,
      measuredLufs: look === "award" ? null : -18.4 - i * 0.7,
      peaks: look === "award" ? [] : fakePeaks(secs, i + 1),
      thumb: `demo:${look}`,
      filmstrip: null,
      analysis: "done",
    });
    const c = newCue(num, name, id);
    c.id = `c_demo${i}`;
    c.note = note;
    c.end = end;
    c.audio.gainDb = gain;
    if (num === "4") {
      c.audio.fadeInMs = 1000;
      c.audio.fadeOutMs = 1000;
      c.audio.normalize.enabled = true;
      c.audio.map = [{ src: 1, out: "L" }, { src: 2, out: "R" }];
      c.transition = { type: "dissolve", durationMs: 500 };
      c.picture.contrast = 6;
    }
    if (num === "8") c.gotoCueId = "c_demo6";
    cues.push(c);
  });
  const bankNames: [string, number][] = [["F1", 0], ["F4", 6]];
  return {
    format: "trapdoor",
    version: 1,
    show: {
      name: "Sample Ceremony 2026 (fictional)",
      frameRate: rate,
      output: { display: 2, width: 1920, height: 1080, background: "#000000", hideCursor: true },
      audio: { device: "auto", masterDb: 0 },
      transitionDefault: { type: "dissolve", durationMs: 500 },
    },
    media,
    cues,
    bank: bankNames.map(([slot, i]) => ({ slot, cueId: cues[i].id })),
    remote: { oscPort: 53000, httpPort: 8780, enabled: true },
  };
}

interface OnAir {
  cueId: string;
  pos: number; // seconds from cue IN
  phase: ProgramState["phase"];
  loops: number;
  fade: number;
  fadeTarget: number;
  fadeRate: number;
  clearAtZero: boolean;
  loopOverride: boolean | null;
}

export class SimEngine {
  show: Show;
  program: OnAir | null = null;
  standby: string | null;
  black = false;
  locked = false;
  rev = 1;
  private last = performance.now();
  private playoutL = new Set<Listener<PlayoutState>>();
  private showL = new Set<Listener<Show>>();
  private logL = new Set<Listener<LogEntry>>();

  constructor(show: Show) {
    this.show = show;
    this.standby = show.cues[0]?.id ?? null;
    setInterval(() => this.tick(), 33);
  }

  onPlayout(f: Listener<PlayoutState>) {
    this.playoutL.add(f);
    return () => this.playoutL.delete(f);
  }
  onShow(f: Listener<Show>) {
    this.showL.add(f);
    return () => this.showL.delete(f);
  }
  onLog(f: Listener<LogEntry>) {
    this.logL.add(f);
    return () => this.logL.delete(f);
  }
  private log(msg: string, level = "info") {
    this.logL.forEach((f) => f({ level, msg, t: performance.now() / 1000 }));
  }
  private bump() {
    this.rev++;
    const s = structuredClone(this.show);
    this.showL.forEach((f) => f(s));
  }

  private cue(id: string | null) {
    return this.show.cues.find((c) => c.id === id) ?? null;
  }
  private durFrames(c: Cue): number | null {
    if (c.outFrame != null) return c.outFrame - c.inFrame;
    const m = this.show.media.find((m) => m.id === c.mediaId);
    if (!m) return null;
    if (m.kind === "still") return c.stillFrames ?? null;
    return m.durationFrames - c.inFrame;
  }
  private nextAfter(id: string): string | null {
    const i = this.show.cues.findIndex((c) => c.id === id);
    return this.show.cues[i + 1]?.id ?? null;
  }

  private take(id: string, moveStandby = true) {
    const c = this.cue(id);
    if (!c) return;
    const fadeIn = c.audio.fadeInMs / 1000;
    this.program = {
      cueId: id,
      pos: 0,
      phase: c.start === "pauseAtStart" ? "cued" : "playing",
      loops: 0,
      fade: fadeIn > 0 ? 0 : 1,
      fadeTarget: 1,
      fadeRate: fadeIn > 0 ? 1 / fadeIn : 999,
      clearAtZero: false,
      loopOverride: null,
    };
    this.log(`GO ${c.number} ${c.name}`);
    if (moveStandby) this.standby = this.nextAfter(id);
  }

  handle(cmd: Command): void {
    const edit = ["addCue", "updateCue", "deleteCue", "moveCue", "setBank", "updateShow"].includes(cmd.cmd);
    if (this.locked && edit) throw new Error("Show is locked");
    const p = this.program;
    switch (cmd.cmd) {
      case "go":
        if (p?.phase === "cued") {
          p.phase = "playing";
          return;
        }
        if (this.standby) this.take(this.standby);
        break;
      case "take":
        this.take(cmd.cueId);
        break;
      case "takeNumber": {
        const c = this.show.cues.find((c) => c.number === cmd.number);
        if (c) this.take(c.id);
        break;
      }
      case "standby":
        this.standby = cmd.cueId;
        break;
      case "standbyNext":
      case "standbyPrev": {
        const i = this.show.cues.findIndex((c) => c.id === this.standby);
        const j = cmd.cmd === "standbyNext" ? Math.min(i + 1, this.show.cues.length - 1) : Math.max(i - 1, 0);
        this.standby = this.show.cues[j]?.id ?? null;
        break;
      }
      case "pause":
        if (p?.phase === "playing") p.phase = "paused";
        break;
      case "resume":
        if (p && (p.phase === "paused" || p.phase === "cued")) p.phase = "playing";
        break;
      case "togglePause":
        if (p?.phase === "playing") p.phase = "paused";
        else if (p && (p.phase === "paused" || p.phase === "cued")) p.phase = "playing";
        break;
      case "stop":
        this.program = null;
        this.log("STOP");
        break;
      case "panic":
        if (p) {
          p.fadeTarget = 0;
          p.fadeRate = 1;
          p.clearAtZero = true;
          p.phase = "fadingOut";
          this.log("PANIC");
        }
        break;
      case "gotoRemain":
        if (p) {
          const c = this.cue(p.cueId)!;
          const d = this.durFrames(c);
          if (d != null) p.pos = Math.max(0, framesToSeconds(d, this.show.show.frameRate) - cmd.seconds);
          this.log(`jump to -${cmd.seconds}s`);
        }
        break;
      case "scrub":
        if (p && !this.locked) {
          const d = this.durFrames(this.cue(p.cueId)!);
          if (d != null) p.pos = framesToSeconds(d, this.show.show.frameRate) * cmd.ratio;
        }
        break;
      case "toggleLoop":
        if (p) {
          const c = this.cue(p.cueId)!;
          p.loopOverride = !(p.loopOverride ?? c.end === "loop");
        }
        break;
      case "black":
        this.black = cmd.on;
        break;
      case "toggleBlack":
        this.black = !this.black;
        break;
      case "bank": {
        const b = this.show.bank.find((b) => b.slot.toLowerCase() === cmd.slot.toLowerCase());
        if (!b) throw new Error(`bank slot ${cmd.slot} is empty`);
        this.take(b.cueId, false);
        break;
      }
      case "lock":
        this.locked = cmd.on;
        this.log(cmd.on ? "show locked" : "show unlocked");
        break;
      case "addCue": {
        const i = cmd.after ? this.show.cues.findIndex((c) => c.id === cmd.after) + 1 : this.show.cues.length;
        this.show.cues.splice(i, 0, cmd.cue);
        if (!this.standby) this.standby = cmd.cue.id;
        this.bump();
        break;
      }
      case "updateCue": {
        const i = this.show.cues.findIndex((c) => c.id === cmd.cue.id);
        if (i >= 0) this.show.cues[i] = cmd.cue;
        this.bump();
        break;
      }
      case "deleteCue":
        if (p?.cueId === cmd.cueId) throw new Error("cannot delete the cue on air");
        this.show.cues = this.show.cues.filter((c) => c.id !== cmd.cueId);
        this.show.bank = this.show.bank.filter((b) => b.cueId !== cmd.cueId);
        if (this.standby === cmd.cueId) this.standby = this.show.cues[0]?.id ?? null;
        this.bump();
        break;
      case "moveCue": {
        const i = this.show.cues.findIndex((c) => c.id === cmd.cueId);
        const [c] = this.show.cues.splice(i, 1);
        this.show.cues.splice(Math.min(cmd.toIndex, this.show.cues.length), 0, c);
        this.bump();
        break;
      }
      case "setBank":
        this.show.bank = this.show.bank.filter((b) => b.slot.toLowerCase() !== cmd.slot.toLowerCase());
        if (cmd.cueId) this.show.bank.push({ slot: cmd.slot, cueId: cmd.cueId });
        this.bump();
        break;
      case "updateShow":
        this.show.show = cmd.settings;
        this.bump();
        break;
    }
  }

  private tick() {
    const now = performance.now();
    const dt = (now - this.last) / 1000;
    this.last = now;
    const rate = this.show.show.frameRate;
    const p = this.program;
    let prog: ProgramState | null = null;
    if (p) {
      const c = this.cue(p.cueId);
      if (!c) {
        this.program = null;
      } else {
        // fades
        if (p.fade !== p.fadeTarget) {
          const step = p.fadeRate * dt;
          p.fade = p.fade < p.fadeTarget ? Math.min(p.fadeTarget, p.fade + step) : Math.max(p.fadeTarget, p.fade - step);
          if (p.fade === 0 && p.clearAtZero) this.program = null;
        }
        const d = this.durFrames(c);
        const dSec = d != null ? framesToSeconds(d, rate) : null;
        if (p.phase === "playing" || p.phase === "fadingOut") p.pos += dt * c.speed;
        const looping = p.loopOverride ?? c.end === "loop";
        if (dSec != null && p.pos >= dSec && p.phase === "playing") {
          if (looping && (p.loopOverride === true || c.loopCount === 0 || p.loops + 1 < c.loopCount)) {
            p.pos -= dSec;
            p.loops++;
          } else if (c.end === "hold") {
            p.pos = dSec;
            p.phase = "hold";
          } else if (c.end === "next") {
            const n = this.nextAfter(c.id);
            if (n) this.take(n);
            else this.program = null;
          } else if (c.end === "goto" && c.gotoCueId) {
            this.take(c.gotoCueId);
          } else {
            this.program = null;
          }
        }
        if (this.program) {
          const q = this.program;
          const qc = this.cue(q.cueId)!;
          const qd = this.durFrames(qc);
          const el = Math.floor(q.pos * fps(rate) + 1e-6);
          const elapsed = Math.min(el, qd ?? el);
          prog = {
            cueId: qc.id,
            number: qc.number,
            name: qc.name,
            phase: q.phase,
            end: qc.end,
            looping: q.loopOverride ?? qc.end === "loop",
            elapsedFrames: elapsed,
            remainFrames: qd != null ? Math.max(0, qd - elapsed) : null,
            durationFrames: qd,
            progress: qd ? Math.min(1, elapsed / qd) : null,
            elapsedTc: formatFrames(elapsed, rate),
            remainTc: formatFrames(qd != null ? Math.max(0, qd - elapsed) : null, rate),
          };
        }
      }
    }
    const sb = this.cue(this.standby);
    const playing = prog && (prog.phase === "playing" || prog.phase === "fadingOut");
    const lv = playing ? 0.55 + 0.12 * Math.sin(now / 90) : 0;
    const st: PlayoutState = {
      program: prog,
      standbyCueId: this.standby,
      standbyReady: !!sb,
      black: this.black,
      locked: this.locked,
      droppedFrames: 0,
      level: [lv, lv * 0.93],
      showRev: this.rev,
      standbyName: sb?.name ?? null,
      standbyNumber: sb?.number ?? null,
    };
    this.playoutL.forEach((f) => f(st));
  }
}
