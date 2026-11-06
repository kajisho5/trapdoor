import { useRef } from "react";
import { useUI, useT } from "../store";
import { cueDuration, mediaOf, send } from "../hooks";
import { fps, formatFrames, framesToSeconds } from "../lib/tc";
import { Keycap, Thumb, Wave } from "./ui";
import type { Cue, Media } from "../types";

function Monitor({
  label,
  sub,
  color,
  media,
  cue,
  second,
  black,
}: {
  label: string;
  sub: string;
  color: string;
  media: Media | null;
  cue: Cue | null;
  second?: number;
  black?: boolean;
}) {
  const show = useUI((s) => s.show);
  const rate = show?.show.frameRate ?? "59.94";
  const total = media ? framesToSeconds(media.durationFrames, rate) : 0;
  return (
    <div className="min-w-0 flex-1">
      <div className="flex h-[27px] items-center justify-between px-2" style={{ background: color }}>
        <span className="mono text-[14px] font-bold tracking-[1px] text-[#0e0f10]">{label}</span>
        <span className="mono truncate pl-2 text-[11px] font-semibold uppercase text-[#0e0f10]">{sub}</span>
      </div>
      <div className="relative aspect-video border-[3px] bg-black" style={{ borderColor: color }}>
        {cue && !black && <Thumb media={media} strip={second !== undefined} second={second} totalSeconds={total} className="absolute inset-0" />}
        <div className="pointer-events-none absolute inset-[5%] border border-dashed border-white/25" />
        {black && <div className="absolute inset-0 flex items-center justify-center mono text-[13px] font-bold text-sb">BLACK</div>}
      </div>
      <div className="mt-2.5 truncate text-[15px] font-bold">{cue ? `${cue.number}  ${cue.name}` : "—"}</div>
    </div>
  );
}

function Btn({
  children,
  sub,
  k,
  onClick,
  onDown,
  onUp,
  className = "",
  big,
}: {
  children: React.ReactNode;
  sub?: string;
  k?: string;
  onClick?: () => void;
  onDown?: () => void;
  onUp?: () => void;
  className?: string;
  big?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      onPointerDown={onDown}
      onPointerUp={onUp}
      onPointerLeave={onUp}
      className={`relative flex flex-col items-center justify-center border-[1.5px] mono font-bold ${className}`}
    >
      <span className={big ? "text-[44px] leading-none" : "text-[17px]"}>{children}</span>
      {sub && <span className={`mt-1 font-[IBM_Plex_Sans_Condensed] text-[11px] font-normal opacity-75 ${big ? "mt-3" : ""}`}>{sub}</span>}
      {k && <Keycap k={k} dark={big} />}
    </button>
  );
}

export function PlayoutPanel() {
  const t = useT();
  const show = useUI((s) => s.show);
  const playout = useUI((s) => s.playout);
  const selected = useUI((s) => s.selectedCueId);
  const rate = show?.show.frameRate ?? "59.94";
  const p = playout?.program ?? null;
  const pCue = show?.cues.find((c) => c.id === p?.cueId) ?? null;
  const sCue = show?.cues.find((c) => c.id === playout?.standbyCueId) ?? null;
  const pMedia = mediaOf(pCue, show?.media);
  const sMedia = mediaOf(sCue, show?.media);
  const stopTimer = useRef<number | null>(null);
  const locked = playout?.locked ?? false;

  const remainFrames = p?.remainFrames ?? null;
  const remainSec = remainFrames != null ? framesToSeconds(remainFrames, rate) : null;
  const warn = remainSec != null && remainSec <= 10 && p?.phase === "playing";
  const prog = p?.progress ?? 0;
  const dur = p?.durationFrames ?? null;
  const durSec = dur != null ? framesToSeconds(dur, rate) : null;
  const fadeOut = pCue ? pCue.audio.fadeOutMs / 1000 : 0;
  const p100 = 100 / fps(rate);
  const inSec = pCue ? framesToSeconds(pCue.inFrame, rate) : 0;
  const curSec = p ? inSec + framesToSeconds(p.elapsedFrames, rate) : undefined;

  const endLabel = pCue ? (pCue.end === "goto" ? t("end.goto", { n: show?.cues.find((c) => c.id === pCue.gotoCueId)?.number ?? "?" }) : t(`end.${pCue.end}`)) : "—";
  const remainColor = !p ? "text-dim" : remainSec === 0 && p.phase !== "hold" ? "text-pgm" : warn ? "text-sb" : "text-ink";

  return (
    <section className="flex w-[42%] min-w-[640px] max-w-[810px] shrink-0 flex-col border-l border-line2 bg-panel px-6 pb-4 pt-6">
      <div className="flex gap-[18px]">
        <Monitor
          label={t("preview")}
          sub={sCue ? (playout?.standbyReady ? t("cued") : t("notReady")) : "—"}
          color="#2fb344"
          media={sMedia}
          cue={sCue}
        />
        <Monitor label={t("program")} sub={p ? t("onAirSub") : t("nothing")} color="#e5383b" media={pMedia} cue={pCue} second={curSec} black={playout?.black} />
      </div>
      <div className="mt-1 flex justify-between gap-[18px] mono text-[13px] text-sub">
        <div className="flex flex-1 justify-between">
          <span>IN {formatFrames(sCue?.inFrame ?? null, rate)}</span>
          <span>DUR {formatFrames(sCue ? cueDuration(sCue, sMedia) : null, rate)}</span>
        </div>
        <div className="flex flex-1 justify-between">
          <span>{p?.elapsedTc ?? "--:--:--.--"}</span>
          <span>/ {formatFrames(dur, rate)}</span>
        </div>
      </div>

      <div className="mt-5 flex h-[128px] shrink-0 items-center border border-line2 bg-[#0b0c0d] px-6">
        <div className="flex-1">
          <div className="mono text-[13px] font-bold tracking-[2px] text-sub">{t("remain")}</div>
          <div className={`mono text-[70px] font-bold leading-[1.05] ${remainColor} ${remainSec === 0 && p?.phase === "playing" ? "animate-pulse" : ""}`}>
            {p?.remainTc ?? "--:--:--.--"}
          </div>
        </div>
        <div className="grid w-[200px] grid-cols-[70px_1fr] gap-y-3 text-right">
          <span className="mono text-left text-[11px] text-dim">{t("elapsed")}</span>
          <span className="mono text-[14px] text-sub">{p?.elapsedTc ?? "—"}</span>
          <span className="mono text-left text-[11px] text-dim">{t("atEnd")}</span>
          <span className="truncate text-[14px]">{p?.looping ? t("end.loop") : endLabel}</span>
          <span className="mono text-left text-[11px] text-dim">{t("next")}</span>
          <span className="truncate text-[14px] text-sb">{playout?.standbyNumber ? `${playout.standbyNumber} ${playout.standbyName}` : "—"}</span>
        </div>
      </div>

      {/* Waveform scrub bar */}
      <div className="relative mt-7 h-[48px] shrink-0">
        {durSec != null &&
          [30, 20, 10].map((s) =>
            s < durSec ? (
              <div key={s} className="absolute -top-[20px] h-[68px]" style={{ left: `${(1 - s / durSec) * 100}%` }}>
                <div className="mono -translate-x-1/2 text-[10px] text-sb">-{s}s</div>
                <div className="mx-auto mt-[3px] h-[52px] w-px bg-sb/70" />
              </div>
            ) : null,
          )}
        <div
          className={`relative h-full overflow-hidden border border-line2 bg-[#08090a] ${locked ? "" : "cursor-pointer"}`}
          onClick={(e) => {
            if (locked || !p) return;
            const r = (e.currentTarget as HTMLDivElement).getBoundingClientRect();
            send({ cmd: "scrub", ratio: (e.clientX - r.left) / r.width });
          }}
        >
          <div className="absolute inset-y-0 left-0 bg-pgm/20" style={{ width: `${prog * 100}%` }} />
          {pMedia?.peaks && pCue && (
            <Wave
              peaks={pMedia.peaks}
              from={Math.floor(pCue.inFrame * p100)}
              to={pCue.outFrame != null ? Math.floor(pCue.outFrame * p100) : undefined}
              played={prog}
              color="#6e757c"
              bar={2}
              className="absolute inset-x-[3px] inset-y-[4px] h-[38px] w-[calc(100%-6px)]"
            />
          )}
          <div className="absolute inset-x-0 top-1/2 h-px bg-line2/60" />
          {durSec != null && fadeOut > 0 && <div className="hatch absolute inset-y-0 right-0" style={{ width: `${(fadeOut / durSec) * 100}%` }} />}
          {p && <div className="absolute inset-y-0 w-[1.5px] bg-white" style={{ left: `${prog * 100}%` }} />}
          <span className="mono absolute right-2 top-1 text-[9px] text-dim">{pMedia?.audioChannels ? "L+R" : ""}</span>
        </div>
        {p && <div className="absolute -top-[9px] h-0 w-0 -translate-x-1/2 border-x-[5px] border-t-[7px] border-x-transparent border-t-ink" style={{ left: `${prog * 100}%` }} />}
      </div>
      <div className="mt-1.5 flex justify-between mono text-[10px] text-dim">
        <span>IN {formatFrames(pCue?.inFrame ?? null, rate)}</span>
        <span>OUT {formatFrames(pCue ? (pCue.inFrame + (dur ?? 0)) : null, rate)}</span>
      </div>

      <div className="mt-auto flex gap-3 pt-3">
        <Btn big k="SPACE" sub={t("goSub")} onClick={() => send({ cmd: "go" })} className="h-[116px] w-[300px] shrink-0 border-pvw bg-pvw text-[#04140b]">
          {t("go")}
        </Btn>
        <div className="grid flex-1 grid-cols-15 grid-rows-2 gap-x-2.5 gap-y-3">
          <Btn k="ENTER" sub={t("takeSub")} onClick={() => selected && send({ cmd: "take", cueId: selected })} className="col-span-5 border-line2 bg-panel2">
            {t("take")}
          </Btn>
          <Btn k="P" onClick={() => send({ cmd: "togglePause" })} className="col-span-5 border-line2 bg-panel2">
            {p?.phase === "paused" || p?.phase === "cued" ? t("resume") : t("pause")}
          </Btn>
          <Btn
            k="S"
            sub={t("stopSub")}
            onDown={() => {
              stopTimer.current = window.setTimeout(() => send({ cmd: "stop" }), 500);
            }}
            onUp={() => {
              if (stopTimer.current) clearTimeout(stopTimer.current);
              stopTimer.current = null;
            }}
            className="col-span-5 border-line2 bg-panel2"
          >
            {t("stop")}
          </Btn>
          {[30, 20, 10].map((s, i) => (
            <Btn key={s} k={String(3 - i)} onClick={() => send({ cmd: "gotoRemain", seconds: s })} className="col-span-3 border-line2 bg-panel2 text-sb">
              <span className="text-[15px]">-{s}</span>
            </Btn>
          ))}
          <Btn k="L" onClick={() => send({ cmd: "toggleLoop" })} className={`col-span-3 border-line2 ${p?.looping ? "bg-sel text-[#0e0f10]" : "bg-panel2"}`}>
            <span className="text-[15px]">{t("loop")}</span>
          </Btn>
          <Btn sub="Esc×2" onClick={() => send({ cmd: "panic" })} className="col-span-3 border-2 border-pgm bg-[#3a1214] text-pgm">
            <span className="text-[15px]">{t("panic")}</span>
          </Btn>
        </div>
      </div>
    </section>
  );
}
