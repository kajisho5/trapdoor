import { useEffect, useState } from "react";
import { engine, type Engine } from "./lib/ipc";
import { useUI } from "./store";
import type { Command, Cue, Media } from "./types";

let resolved: Engine | null = null;
engine().then((e) => (resolved = e));

export function useEngine(): Engine | null {
  const [e, setE] = useState<Engine | null>(resolved);
  useEffect(() => {
    if (!e) engine().then(setE);
  }, [e]);
  return e;
}

export async function send(cmd: Command) {
  const e = await engine();
  try {
    await e.command(cmd);
  } catch (err) {
    useUI.getState().flash(String((err as Error)?.message ?? err));
  }
}

export function mediaOf(cue: Cue | null | undefined, media: Media[] | undefined): Media | null {
  if (!cue || !media) return null;
  return media.find((m) => m.id === cue.mediaId) ?? null;
}

export function cueDuration(cue: Cue, m: Media | null): number | null {
  if (cue.outFrame != null) return cue.outFrame - cue.inFrame;
  if (!m) return null;
  if (m.kind === "still") return cue.stillFrames ?? null;
  return m.durationFrames > 0 ? m.durationFrames - cue.inFrame : null;
}

/** Peaks index range for a cue's IN..OUT. */
export function peakRange(cue: Cue, m: Media | null, rateFps: number): [number, number | undefined] {
  const p = 100 / rateFps;
  const a = Math.floor(cue.inFrame * p);
  const b = cue.outFrame != null ? Math.floor(cue.outFrame * p) : undefined;
  return [a, b ?? (m ? undefined : undefined)];
}
