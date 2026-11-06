import { useEffect, useState } from "react";
import { useUI, useT, type Tab } from "../store";
import { cueDuration, mediaOf, send, useEngine } from "../hooks";
import { formatFrames, parseTimecode, shortDur } from "../lib/tc";
import { Thumb, Wave } from "./ui";
import type { Cue, EndAction, Fit } from "../types";

export const BANK_SLOTS = [
  "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
  "S-F1", "S-F2", "S-F3", "S-F4",
];

function Tabs() {
  const t = useT();
  const tab = useUI((s) => s.tab);
  const setTab = useUI((s) => s.setTab);
  const items: [Tab, string][] = [
    ["bank", t("tabBank")],
    ["inspector", t("tabInspector")],
    ["media", t("tabMedia")],
    ["log", t("tabLog")],
  ];
  return (
    <div className="flex h-[46px] items-end gap-1.5 border-b border-line px-4">
      {items.map(([k, label]) => (
        <button
          key={k}
          onClick={() => setTab(k)}
          className={`relative h-10 px-5 text-[14px] ${tab === k ? "bg-panel2 font-bold text-ink" : "text-sub"}`}
        >
          {tab === k && <span className="absolute inset-x-0 top-0 h-[3px] bg-sb" />}
          {label}
        </button>
      ))}
      <div className="flex-1" />
      <TabHint />
    </div>
  );
}

function TabHint() {
  const t = useT();
  const tab = useUI((s) => s.tab);
  const show = useUI((s) => s.show);
  const td = show?.show.transitionDefault;
  if (tab === "bank") return <span className="pb-3 text-[12px] text-sub">{t("bankHint", { t: td?.type === "dissolve" ? `${t("dissolve")} ${(td.durationMs / 1000).toFixed(1)}s` : t("cut") })}</span>;
  if (tab === "inspector") return <span className="pb-3 text-[12px] text-sub">{t("nonDestructive")}</span>;
  return null;
}

function ClipBank() {
  const t = useT();
  const show = useUI((s) => s.show);
  const playout = useUI((s) => s.playout);
  const rate = show?.show.frameRate ?? "59.94";
  return (
    <div className="grid flex-1 grid-cols-8 grid-rows-2 gap-2.5 p-4 pt-3">
      {BANK_SLOTS.map((slot) => {
        const b = show?.bank.find((x) => x.slot.toLowerCase() === slot.toLowerCase());
        const cue = show?.cues.find((c) => c.id === b?.cueId) ?? null;
        const m = mediaOf(cue, show?.media);
        const onAir = cue && playout?.program?.cueId === cue.id;
        const key = slot.replace("S-", "⇧");
        const drop = (e: React.DragEvent) => {
          e.preventDefault();
          const id = e.dataTransfer.getData("text/trapdoor-cue");
          if (id) send({ cmd: "setBank", slot, cueId: id });
        };
        if (!cue)
          return (
            <div key={slot} onDragOver={(e) => e.preventDefault()} onDrop={drop} className="relative flex items-center justify-center border border-dashed border-line2 text-[12px] text-dim">
              <span className="mono absolute left-2.5 top-2 text-[11px] font-bold">{key}</span>
              {t("bankEmpty")}
            </div>
          );
        const warn = cue.color === "yellow";
        return (
          <button
            key={slot}
            onDragOver={(e) => e.preventDefault()}
            onDrop={drop}
            onClick={() => send({ cmd: "bank", slot })}
            onContextMenu={(e) => {
              e.preventDefault();
              if (!playout?.locked) send({ cmd: "setBank", slot, cueId: null });
            }}
            className={`relative flex min-h-0 flex-col overflow-hidden border bg-panel2 text-left ${onAir ? "border-2 border-pgm" : warn ? "border-[1.5px] border-sb" : "border-line2"}`}
            title="Right-click: clear slot"
          >
            <Thumb media={m} className="h-[64%] w-full" />
            <span className="mono absolute left-1.5 top-1.5 bg-black/70 px-1.5 text-[11px] font-bold text-white">{key}</span>
            <div className="flex flex-1 flex-col justify-center px-2.5">
              <div className="flex items-baseline justify-between gap-2">
                <span className="truncate text-[13px] font-medium">{cue.name}</span>
                <span className="mono shrink-0 text-[11px] text-sub">{shortDur(cueDuration(cue, m), rate)}</span>
              </div>
              {m?.peaks && m.peaks.length > 0 && <Wave peaks={m.peaks} bar={2} played={onAir ? playout?.program?.progress ?? 0 : 0} className="mt-1 h-[9px] w-full" />}
            </div>
          </button>
        );
      })}
    </div>
  );
}

// ---------- Inspector ----------

function TcField({ label, frames, onCommit, disabled, w = 136 }: { label: string; frames: number | null; onCommit?: (f: number | null) => void; disabled?: boolean; w?: number }) {
  const show = useUI((s) => s.show);
  const rate = show?.show.frameRate ?? "59.94";
  const [v, setV] = useState(formatFrames(frames, rate));
  useEffect(() => setV(formatFrames(frames, rate)), [frames, rate]);
  return (
    <label style={{ width: w }}>
      <span className="lbl">{label}</span>
      <input
        className="field"
        value={v}
        disabled={disabled || !onCommit}
        onChange={(e) => setV(e.target.value)}
        onBlur={() => {
          const f = parseTimecode(v, rate);
          if (f == null) setV(formatFrames(frames, rate));
          else onCommit?.(f);
        }}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
      />
    </label>
  );
}

function NumField({ label, value, onCommit, w = 96, step = 1, suffix = "", disabled }: { label: string; value: number; onCommit: (n: number) => void; w?: number; step?: number; suffix?: string; disabled?: boolean }) {
  const [v, setV] = useState(String(value));
  useEffect(() => setV(String(value)), [value]);
  return (
    <label style={{ width: w }}>
      <span className="lbl">{label}</span>
      <div className="relative">
        <input
          className="field pr-7"
          value={v}
          disabled={disabled}
          inputMode="decimal"
          onChange={(e) => setV(e.target.value)}
          onBlur={() => {
            const n = Number(v);
            if (Number.isFinite(n)) onCommit(Math.round(n / step) * step);
            else setV(String(value));
          }}
          onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        />
        {suffix && <span className="mono pointer-events-none absolute right-2 top-[7px] text-[11px] text-dim">{suffix}</span>}
      </div>
    </label>
  );
}

function Seg<T extends string>({ label, value, options, onChange, disabled }: { label: string; value: T; options: [T, string][]; onChange: (v: T) => void; disabled?: boolean }) {
  return (
    <div>
      <span className="lbl">{label}</span>
      <div className="seg">
        {options.map(([k, l]) => (
          <button key={k} aria-pressed={value === k} disabled={disabled} onClick={() => onChange(k)}>
            {l}
          </button>
        ))}
      </div>
    </div>
  );
}

function Slider({ label, value, min, max, step = 1, onChange, fmt, w = 150, disabled }: { label: string; value: number; min: number; max: number; step?: number; onChange: (v: number) => void; fmt?: (v: number) => string; w?: number; disabled?: boolean }) {
  return (
    <div style={{ width: w + 60 }}>
      <div className="flex justify-between">
        <span className="lbl">{label}</span>
        <span className="mono text-[12px]">{fmt ? fmt(value) : value}</span>
      </div>
      <input type="range" min={min} max={max} step={step} value={value} disabled={disabled} onChange={(e) => onChange(Number(e.target.value))} style={{ width: w + 60 }} />
    </div>
  );
}

function Section({ title, children, className = "" }: { title: string; children: React.ReactNode; className?: string }) {
  return (
    <div className={`min-w-0 ${className}`}>
      <div className="mb-2 flex items-center gap-3">
        <span className="mono text-[11px] font-bold tracking-[1.5px] text-sub">{title}</span>
        <div className="h-px flex-1 bg-line" />
      </div>
      <div className="flex flex-wrap gap-x-3 gap-y-2">{children}</div>
    </div>
  );
}

function CueInspector() {
  const t = useT();
  const show = useUI((s) => s.show);
  const selected = useUI((s) => s.selectedCueId);
  const playout = useUI((s) => s.playout);
  const select = useUI((s) => s.select);
  const cue = show?.cues.find((c) => c.id === selected) ?? null;
  const m = mediaOf(cue, show?.media);
  const locked = playout?.locked ?? false;
  if (!cue || !show) return <div className="flex flex-1 items-center justify-center text-[13px] text-dim">{t("noCue")}</div>;

  const upd = (patch: (c: Cue) => void) => {
    const c = structuredClone(cue);
    patch(c);
    send({ cmd: "updateCue", cue: c });
  };
  const gainApplied = (() => {
    let g = cue.audio.gainDb + show.show.audio.masterDb;
    if (cue.audio.normalize.enabled && m?.measuredLufs != null) g += cue.audio.normalize.targetLufs - m.measuredLufs;
    return Math.max(-60, Math.min(12, g));
  })();
  const out = cue.outFrame ?? (m ? m.durationFrames : null);
  const ends: [EndAction, string][] = [
    ["black", t("end.black")],
    ["hold", t("end.hold")],
    ["next", t("end.next")],
    ["loop", t("end.loop")],
    ["goto", t("end.goto", { n: "…" })],
  ];
  const fits: [Fit, string][] = [
    ["fit", t("fit.fit")],
    ["fill", t("fit.fill")],
    ["stretch", t("fit.stretch")],
    ["native", t("fit.native")],
  ];
  const ch = Math.max(2, m?.audioChannels ?? 2);
  const map = cue.audio.map.length ? cue.audio.map : [{ src: 1, out: "L" }, { src: 2, out: "R" }];
  const COLORS = ["red", "yellow", "green", "blue", "purple", null] as const;
  const colorHex: Record<string, string> = { red: "#e5383b", yellow: "#f2a900", green: "#2fb344", blue: "#3e9bff", purple: "#a78bfa" };

  return (
    <div className="grid min-h-0 flex-1 grid-cols-[236px_1fr_1fr_1fr] gap-x-7 overflow-y-auto px-4 pt-3">
      <div className="min-w-0">
        <Thumb media={m} className="aspect-video w-full border border-line2" />
        <div className="mt-2 flex items-center gap-2">
          <input className="field w-[52px] text-[16px] font-bold text-sb" value={cue.number} disabled={locked} onChange={(e) => upd((c) => (c.number = e.target.value))} />
          <input className="field flex-1 font-[IBM_Plex_Sans_Condensed] text-[15px] font-bold" value={cue.name} disabled={locked} onChange={(e) => upd((c) => (c.name = e.target.value))} />
        </div>
        <dl className="mt-2 grid grid-cols-[56px_1fr] gap-y-0.5 text-[11px]">
          <dt className="text-dim">File</dt>
          <dd className="truncate text-right text-sub" title={m?.path}>{m?.path.split(/[\\/]/).pop() ?? "—"}</dd>
          <dt className="text-dim">Codec</dt>
          <dd className="mono truncate text-right text-sub">{m?.codec ?? "—"}</dd>
          <dt className="text-dim">Format</dt>
          <dd className="mono text-right text-sub">{m && m.width ? `${m.width}×${m.height} ${m.fps ? m.fps.toFixed(2) + "p" : ""}` : "—"}</dd>
          <dt className="text-dim">Audio</dt>
          <dd className="mono text-right text-sub">{m ? `${m.audioChannels} ch · ${m.measuredLufs != null ? m.measuredLufs.toFixed(1) + " LUFS" : "—"}` : "—"}</dd>
        </dl>
      </div>

      <Section title={t("timing")}>
        <TcField label={t("in")} frames={cue.inFrame} disabled={locked} onCommit={(f) => upd((c) => (c.inFrame = Math.min(f ?? 0, (out ?? Infinity) - 1)))} />
        <TcField label={t("out")} frames={out} disabled={locked} onCommit={(f) => upd((c) => (c.outFrame = f != null && m && f >= m.durationFrames ? null : f))} />
        <TcField label={t("duration")} frames={cueDuration(cue, m)} />
        <Seg label={t("onStart")} value={cue.start} disabled={locked} options={[["play", t("start.play")], ["pauseAtStart", t("start.pauseAtStart")]]} onChange={(v) => upd((c) => (c.start = v))} />
        <Seg label={t("atEndLabel")} value={cue.end} disabled={locked} options={ends} onChange={(v) => upd((c) => (c.end = v))} />
        {cue.end === "goto" && (
          <label className="w-[150px]">
            <span className="lbl">{t("gotoTarget")}</span>
            <select className="field" disabled={locked} value={cue.gotoCueId ?? ""} onChange={(e) => upd((c) => (c.gotoCueId = e.target.value || null))}>
              <option value="">—</option>
              {show.cues.filter((x) => x.id !== cue.id).map((x) => (
                <option key={x.id} value={x.id}>
                  {x.number} {x.name}
                </option>
              ))}
            </select>
          </label>
        )}
        {cue.end === "loop" && <NumField label={t("loops")} value={cue.loopCount} disabled={locked} onCommit={(n) => upd((c) => (c.loopCount = Math.max(0, Math.round(n))))} />}
        <NumField label={t("speed")} value={cue.speed} step={0.01} suffix="×" w={86} disabled={locked} onCommit={(n) => upd((c) => (c.speed = Math.max(0.25, Math.min(2, n))))} />
        <Seg label={t("transitionIn")} value={cue.transition.type} disabled={locked} options={[["cut", t("cut")], ["dissolve", t("dissolve")]]} onChange={(v) => upd((c) => (c.transition = { type: v, durationMs: v === "dissolve" ? c.transition.durationMs || 500 : 0 }))} />
        {cue.transition.type === "dissolve" && <NumField label={t("sec")} value={cue.transition.durationMs / 1000} step={0.05} w={64} disabled={locked} onCommit={(n) => upd((c) => (c.transition.durationMs = Math.round(Math.max(0, Math.min(10, n)) * 1000)))} />}
      </Section>

      <Section title={t("audio")}>
        <Slider label={t("gain")} value={cue.audio.gainDb} min={-60} max={12} step={0.5} w={270} disabled={locked} fmt={(v) => `${v.toFixed(1)} dB`} onChange={(v) => upd((c) => (c.audio.gainDb = v))} />
        <div className="flex w-full items-center gap-2 text-[12px]">
          <input type="checkbox" checked={cue.audio.normalize.enabled} disabled={locked} onChange={(e) => upd((c) => (c.audio.normalize.enabled = e.target.checked))} className="h-4 w-4 accent-[#d0d5da]" />
          <span>{t("normalize")}</span>
          <select className="field w-[112px]" disabled={locked} value={cue.audio.normalize.targetLufs} onChange={(e) => upd((c) => (c.audio.normalize.targetLufs = Number(e.target.value)))}>
            {[-14, -16, -18, -20, -23, -24, -27].map((v) => (
              <option key={v} value={v}>
                {v} LUFS
              </option>
            ))}
          </select>
          {cue.audio.normalize.enabled && <span className="text-[11px] text-sub">{t("applied", { g: (gainApplied - cue.audio.gainDb - show.show.audio.masterDb >= 0 ? "+" : "") + (gainApplied - cue.audio.gainDb - show.show.audio.masterDb).toFixed(1) })}</span>}
        </div>
        <NumField label={t("fadeIn")} value={cue.audio.fadeInMs / 1000} step={0.1} suffix="s" disabled={locked} onCommit={(n) => upd((c) => (c.audio.fadeInMs = Math.round(Math.max(0, Math.min(10, n)) * 1000)))} />
        <NumField label={t("fadeOut")} value={cue.audio.fadeOutMs / 1000} step={0.1} suffix="s" disabled={locked} onCommit={(n) => upd((c) => (c.audio.fadeOutMs = Math.round(Math.max(0, Math.min(10, n)) * 1000)))} />
        <Seg label={t("mute")} value={cue.audio.mute ? "on" : "off"} disabled={locked} options={[["off", t("off")], ["on", t("on")]]} onChange={(v) => upd((c) => (c.audio.mute = v === "on"))} />
        <div className="w-full">
          <span className="lbl">{t("channelMap")}</span>
          <div className="flex items-start gap-6">
            <div className="flex flex-col gap-2">
              {map.slice(0, 2).map((r, i) => (
                <div key={i} className="flex items-center gap-2">
                  <select className="field w-[78px]" disabled={locked} value={r.src} onChange={(e) => upd((c) => { const mm = structuredClone(map); mm[i].src = Number(e.target.value); c.audio.map = mm; })}>
                    {Array.from({ length: ch }, (_, k) => (
                      <option key={k} value={k + 1}>
                        Src {k + 1}
                      </option>
                    ))}
                  </select>
                  <span className="text-dim">⟶</span>
                  <span className="field flex w-[70px] items-center">Out {r.out}</span>
                </div>
              ))}
            </div>
            <Slider label={t("pan")} value={cue.audio.pan} min={-1} max={1} step={0.05} w={70} disabled={locked} fmt={(v) => (Math.abs(v) < 0.01 ? "C" : v < 0 ? `L${Math.round(-v * 100)}` : `R${Math.round(v * 100)}`)} onChange={(v) => upd((c) => (c.audio.pan = v))} />
          </div>
        </div>
      </Section>

      <Section title={t("picture")}>
        <Seg label={t("fit")} value={cue.picture.fit} options={fits} disabled={locked} onChange={(v) => upd((c) => (c.picture.fit = v))} />
        <NumField label={t("scale")} value={Math.round(cue.picture.scale * 100)} suffix="%" w={78} disabled={locked} onCommit={(n) => upd((c) => (c.picture.scale = Math.max(25, Math.min(400, n)) / 100))} />
        <NumField label="X" value={cue.picture.x} w={66} disabled={locked} onCommit={(n) => upd((c) => (c.picture.x = Math.round(n)))} />
        <NumField label="Y" value={cue.picture.y} w={66} disabled={locked} onCommit={(n) => upd((c) => (c.picture.y = Math.round(n)))} />
        <div>
          <span className="lbl">{t("crop")}</span>
          <div className="flex gap-2">
            {(["top", "bottom", "left", "right"] as const).map((k) => (
              <input key={k} className="field w-[50px]" disabled={locked} defaultValue={cue.picture.crop[k]} onBlur={(e) => upd((c) => (c.picture.crop[k] = Math.max(0, Math.round(Number(e.target.value) || 0))))} />
            ))}
          </div>
        </div>
        <NumField label={t("opacity")} value={Math.round(cue.picture.opacity * 100)} suffix="%" w={86} disabled={locked} onCommit={(n) => upd((c) => (c.picture.opacity = Math.max(0, Math.min(100, n)) / 100))} />
        <div>
          <span className="lbl">{t("flip")}</span>
          <div className="seg">
            <button aria-pressed={cue.picture.flipH} disabled={locked} onClick={() => upd((c) => (c.picture.flipH = !c.picture.flipH))}>H</button>
            <button aria-pressed={cue.picture.flipV} disabled={locked} onClick={() => upd((c) => (c.picture.flipV = !c.picture.flipV))}>V</button>
          </div>
        </div>
        {(["brightness", "contrast", "saturation"] as const).map((k) => (
          <Slider key={k} label={t(k)} value={cue.picture[k]} min={-100} max={100} disabled={locked} fmt={(v) => (v > 0 ? `+${v}` : String(v))} onChange={(v) => upd((c) => (c.picture[k] = v))} />
        ))}
        <div className="flex items-end gap-2">
          <button
            disabled={locked}
            onClick={() =>
              upd((c) => {
                c.picture = { fit: "fit", scale: 1, x: 0, y: 0, crop: { top: 0, bottom: 0, left: 0, right: 0 }, opacity: 1, flipH: false, flipV: false, brightness: 0, contrast: 0, saturation: 0 };
              })
            }
            className="h-8 border border-line2 px-3 text-[12px] text-sub"
          >
            {t("resetPicture")}
          </button>
          <div className="flex gap-1.5">
            {COLORS.map((c) => (
              <button
                key={String(c)}
                disabled={locked}
                onClick={() => upd((x) => (x.color = c))}
                className={`h-6 w-6 rounded-full ${cue.color === c ? "ring-2 ring-white" : ""}`}
                style={{ background: c ? colorHex[c] : "#5c6268" }}
                title={c ?? "none"}
              />
            ))}
          </div>
          <button
            disabled={locked}
            onClick={() => {
              send({ cmd: "deleteCue", cueId: cue.id });
              select(null);
            }}
            className="h-8 border border-pgm/60 px-3 text-[12px] text-pgm"
          >
            {t("delete")}
          </button>
        </div>
        <label className="w-full">
          <span className="lbl">{t("note")}</span>
          <input className="field font-[IBM_Plex_Sans_Condensed]" disabled={locked} value={cue.note} onChange={(e) => upd((c) => (c.note = e.target.value))} />
        </label>
      </Section>
    </div>
  );
}

function MediaBrowser() {
  const t = useT();
  const show = useUI((s) => s.show);
  const eng = useEngine();
  const rate = show?.show.frameRate ?? "59.94";
  const media = show?.media ?? [];
  if (!media.length) return <div className="flex flex-1 items-center justify-center text-[13px] text-dim">{t("noMedia")}</div>;
  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-4 pt-2">
      <table className="w-full text-[12px]">
        <thead className="text-left text-sub">
          <tr className="h-7">
            <th className="w-[80px]" />
            <th>{t("mediaPath")}</th>
            <th>{t("mediaInfo")}</th>
            <th>{t("duration")}</th>
            <th>{t("analysis")}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {media.map((m) => (
            <tr key={m.id} className="h-12 border-t border-line">
              <td>
                <Thumb media={m} className="h-[36px] w-[64px] border border-line2" />
              </td>
              <td className="max-w-[520px] truncate pr-4" title={m.path}>{m.path}</td>
              <td className="mono text-sub">{[m.codec, m.width ? `${m.width}×${m.height}` : null, m.fps ? `${m.fps.toFixed(2)}p` : null, m.audioChannels ? `${m.audioChannels}ch` : null].filter(Boolean).join(" · ") || "—"}</td>
              <td className="mono">{m.kind === "still" ? "still" : formatFrames(m.durationFrames, rate)}</td>
              <td className={`mono font-bold ${m.analysis === "done" ? "text-pvw" : m.analysis === "missing" || m.analysis === "failed" ? "text-pgm" : "text-sb"}`}>
                {m.analysis.toUpperCase()}
                {m.fps && show && Math.abs(m.fps - Number(show.show.frameRate)) > 0.02 ? <span className="ml-2 text-sb">FPS≠{show.show.frameRate}</span> : null}
              </td>
              <td className="text-right">
                {(m.analysis === "missing" || m.analysis === "failed") && (
                  <button onClick={() => eng?.relink(m.id)} className="h-7 border border-line2 px-3 text-[12px]">
                    {t("relink")}
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EventLog() {
  const logs = useUI((s) => s.logs);
  return (
    <div className="mono min-h-0 flex-1 overflow-y-auto px-4 pt-2 text-[12px]">
      {[...logs].reverse().map((l, i) => (
        <div key={i} className={l.level === "warn" ? "text-sb" : "text-sub"}>
          {l.t != null ? `${l.t.toFixed(1).padStart(8)}s  ` : ""}
          {l.msg}
        </div>
      ))}
    </div>
  );
}

export function Bottom() {
  const tab = useUI((s) => s.tab);
  return (
    <div className="flex min-w-0 flex-1 flex-col">
      <Tabs />
      {tab === "bank" && <ClipBank />}
      {tab === "inspector" && <CueInspector />}
      {tab === "media" && <MediaBrowser />}
      {tab === "log" && <EventLog />}
    </div>
  );
}
