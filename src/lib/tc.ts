import type { FrameRate } from "../types";

const RATES: Record<FrameRate, [number, number]> = {
  "23.976": [24000, 1001],
  "24": [24, 1],
  "25": [25, 1],
  "29.97": [30000, 1001],
  "30": [30, 1],
  "50": [50, 1],
  "59.94": [60000, 1001],
  "60": [60, 1],
};

export const fps = (r: FrameRate) => RATES[r][0] / RATES[r][1];
export const framesToSeconds = (f: number, r: FrameRate) => (f * RATES[r][1]) / RATES[r][0];
export const secondsToFrames = (s: number, r: FrameRate) => (s <= 0 ? 0 : Math.round(s * fps(r)));

/** HH:MM:SS.FF — same formatting as the engine (wall-clock based). */
export function formatFrames(frames: number | null | undefined, r: FrameRate): string {
  if (frames == null) return "--:--:--.--";
  // Real elapsed time, shown at the nominal frame grid (59.94 reads like a clock).
  const nominal = Math.round(fps(r));
  const n = Math.round(framesToSeconds(frames, r) * nominal);
  const whole = Math.floor(n / nominal);
  const ff = n % nominal;
  const p = (v: number) => String(v).padStart(2, "0");
  return `${p(Math.floor(whole / 3600))}:${p(Math.floor(whole / 60) % 60)}:${p(whole % 60)}.${p(ff)}`;
}

/** Short m:ss for bank buttons. */
export function shortDur(frames: number | null | undefined, r: FrameRate): string {
  if (frames == null) return "∞";
  const s = Math.round(framesToSeconds(frames, r));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Parse "HH:MM:SS.FF", "MM:SS.FF", "SS.FF" or "SS" into frames. Returns null if invalid. */
export function parseTimecode(text: string, r: FrameRate): number | null {
  const t = text.trim();
  const m = t.match(/^(?:(\d+):)?(?:(\d+):)?(\d+)(?:[.;:](\d+))?$/);
  if (!m) return null;
  const parts = [m[1], m[2], m[3]].filter((x) => x !== undefined).map(Number);
  while (parts.length < 3) parts.unshift(0);
  const [h, mi, s] = parts;
  const ff = m[4] ? Number(m[4]) : 0;
  if (mi > 59 || s > 59 || ff >= Math.round(fps(r))) return null;
  const seconds = h * 3600 + mi * 60 + s;
  return secondsToFrames(seconds, r) + ff;
}
