import { useMemo, useState } from "react";
import { useUI, useT } from "../store";
import { cueDuration, mediaOf, send, useEngine } from "../hooks";
import { fps, formatFrames } from "../lib/tc";
import { EndIcon, Thumb, Wave } from "./ui";
import type { Cue } from "../types";

export function CueList() {
  const t = useT();
  const show = useUI((s) => s.show);
  const playout = useUI((s) => s.playout);
  const selected = useUI((s) => s.selectedCueId);
  const select = useUI((s) => s.select);
  const setTab = useUI((s) => s.setTab);
  const eng = useEngine();
  const [dragId, setDragId] = useState<string | null>(null);
  const [overIdx, setOverIdx] = useState<number | null>(null);
  const rate = show?.show.frameRate ?? "59.94";
  const locked = playout?.locked ?? false;
  const cues = show?.cues ?? [];
  const pgm = playout?.program?.cueId ?? null;
  const sb = playout?.standbyCueId ?? null;
  const pgmIdx = cues.findIndex((c) => c.id === pgm);

  const total = useMemo(() => cues.reduce((a, c) => a + (cueDuration(c, mediaOf(c, show?.media)) ?? 0), 0), [cues, show?.media]);
  const issues = useMemo(() => (show?.media ?? []).filter((m) => m.analysis === "missing" || m.analysis === "failed").length, [show?.media]);

  const addCue = async () => {
    if (!eng || locked) return;
    const paths = await eng.pickMedia();
    if (paths.length) await eng.addMedia(paths, selected);
  };

  const endLabel = (c: Cue) => {
    if (c.end === "goto") {
      const g = cues.find((x) => x.id === c.gotoCueId);
      return t("end.goto", { n: g?.number ?? "?" });
    }
    return t(`end.${c.end}`);
  };
  const fadeTxt = (ms: number) => (ms > 0 ? (ms / 1000).toFixed(1) : "—");

  return (
    <section className="flex min-w-0 flex-[1110] flex-col bg-bg">
      <div className="flex h-12 shrink-0 items-center px-5">
        <span className="mono text-[13px] font-bold tracking-[1.5px] text-sub">{t("cueList")}</span>
        <span className="ml-3 text-[12px] text-dim">{t("cuesTotal", { n: cues.length, t: formatFrames(total, rate) })}</span>
        <div className="flex-1" />
        <button onClick={addCue} disabled={locked} className="mr-2 h-7 border border-line2 bg-panel2 px-3 text-[12px] disabled:opacity-40">
          {t("addCue")}
        </button>
        <button
          onClick={() => setTab("media")}
          className={`h-7 border border-line2 bg-panel2 px-3 text-[12px] ${issues ? "text-pgm" : "text-pvw"}`}
        >
          {issues ? t("preflightIssues", { n: issues }) : t("preflightOk")}
        </button>
      </div>
      <div className="grid h-[30px] shrink-0 grid-cols-[72px_114px_1fr_140px_140px_130px_80px] items-center border-b border-line bg-panel pl-6 text-[11px] text-sub">
        <span>#</span>
        <span>{t("colMedia")}</span>
        <span>{t("colName")}</span>
        <span>{t("colDuration")}</span>
        <span>{t("colEnd")}</span>
        <span>{t("colFade")}</span>
        <span>{t("colGain")}</span>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto" onDragOver={(e) => e.preventDefault()}>
        {cues.length === 0 && <div className="p-10 text-center text-[13px] text-dim">{t("dropHere")}</div>}
        {cues.map((c, i) => {
          const m = mediaOf(c, show?.media);
          const d = cueDuration(c, m);
          const isPgm = c.id === pgm;
          const isSb = c.id === sb;
          const done = pgmIdx >= 0 ? i < pgmIdx : false;
          const isSel = c.id === selected;
          const p100 = 100 / fps(rate);
          const peaks = m?.peaks;
          return (
            <div
              key={c.id}
              draggable={!locked}
              onDragStart={(e) => {
                setDragId(c.id);
                e.dataTransfer.setData("text/trapdoor-cue", c.id);
              }}
              onDragEnd={() => {
                setDragId(null);
                setOverIdx(null);
              }}
              onDragOver={(e) => {
                if (dragId) {
                  e.preventDefault();
                  setOverIdx(i);
                }
              }}
              onDrop={(e) => {
                e.preventDefault();
                if (dragId && dragId !== c.id) send({ cmd: "moveCue", cueId: dragId, toIndex: i });
                setDragId(null);
                setOverIdx(null);
              }}
              onClick={() => select(c.id)}
              onDoubleClick={() => send({ cmd: "standby", cueId: c.id })}
              className={`relative grid h-[68px] grid-cols-[72px_114px_1fr_140px_140px_130px_80px] items-center border-b border-line pl-6 ${
                isPgm ? "bg-[#2a1416]" : i % 2 ? "bg-[#111214]" : "bg-bg"
              } ${overIdx === i ? "border-t-2 border-t-sel" : ""}`}
            >
              {isPgm && <div className="absolute inset-y-0 left-0 w-1.5 bg-pgm" />}
              {isSb && <div className="pointer-events-none absolute inset-[1.5px] border-[2.5px] border-sb" />}
              {isSb && <div className="absolute inset-y-0 left-0 w-1.5 bg-sb" />}
              {isSel && !isSb && !isPgm && <div className="pointer-events-none absolute inset-[1px] border border-sel/60" />}
              <span className={`mono text-[20px] font-bold ${isPgm ? "text-pgm" : isSb ? "text-sb" : ""} ${done ? "opacity-40" : ""}`}>{c.number}</span>
              <Thumb media={m} className={`h-[54px] w-[96px] border border-line2 ${done ? "opacity-40" : ""}`} />
              <div className={`relative min-w-0 pr-3 ${done ? "opacity-40" : ""}`}>
                <div className="flex items-center gap-3">
                  <span className={`truncate text-[16px] ${isPgm || isSb ? "font-bold" : "font-medium"}`}>{c.name}</span>
                  {m?.analysis === "missing" && <span className="mono text-[10px] font-bold text-pgm">MISSING</span>}
                  {m?.analysis === "running" && <span className="mono text-[10px] text-dim">ANALYZING…</span>}
                  <div className="flex-1" />
                  {isPgm && <span className="mono shrink-0 bg-pgm px-3 py-0.5 text-[11px] font-bold text-white">{t("onAir")}</span>}
                  {isSb && !isPgm && <span className="mono shrink-0 bg-sb px-3 py-0.5 text-[11px] font-bold text-[#1a1200]">{t("standby")}</span>}
                  {done && <span className="shrink-0 text-[12px] text-sub">{t("played")}</span>}
                </div>
                {c.note && <div className="truncate text-[12px] text-sub">{c.note}</div>}
              </div>
              <span className={`mono text-[15px] ${done ? "opacity-40" : ""}`}>{formatFrames(d, rate)}</span>
              <span className={`flex items-center gap-1.5 text-[13px] ${done ? "opacity-40" : ""}`}>
                <EndIcon end={c.end} />
                {endLabel(c)}
              </span>
              <span className={`mono text-[14px] text-sub ${done ? "opacity-40" : ""}`}>
                {fadeTxt(c.audio.fadeInMs)}/{fadeTxt(c.audio.fadeOutMs)}
              </span>
              <span className={`mono text-[14px] text-sub ${done ? "opacity-40" : ""}`}>
                {m && m.audioChannels === 0 && m.kind !== "audio" ? "—" : c.audio.gainDb.toFixed(1)}
              </span>
              {peaks && peaks.length > 0 && (
                <div className="pointer-events-none absolute bottom-[3px] left-[210px] right-6 h-[10px]">
                  <Wave
                    peaks={peaks}
                    from={Math.floor(c.inFrame * p100)}
                    to={c.outFrame != null ? Math.floor(c.outFrame * p100) : undefined}
                    played={isPgm ? playout?.program?.progress ?? 0 : done ? 1 : 0}
                    color={done ? "#2a2c2f" : "#3a3d41"}
                    playedColor={isPgm ? "#e5383b" : "#2a2c2f"}
                    bar={4}
                    className="h-full w-full"
                  />
                </div>
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}
