import { useEffect, useRef, useState } from "react";
import { useUI, useT } from "../store";
import { send } from "../hooks";

const SCALE: [string, number][] = [["0", 0], ["-6", 0.14], ["-12", 0.28], ["-18", 0.42], ["-30", 0.64], ["-48", 0.88]];

/** Linear 0..1 → meter position (0 dB at top, -60 dB at bottom). */
function pos(lin: number) {
  if (lin <= 0) return 0;
  const db = 20 * Math.log10(lin);
  return Math.max(0, Math.min(1, 1 + db / 60));
}

export function Meters() {
  const t = useT();
  const playout = useUI((s) => s.playout);
  const show = useUI((s) => s.show);
  const lv = playout?.level ?? [0, 0];
  const peaks = useRef<[number, number]>([0, 0]);
  const [pk, setPk] = useState<[number, number]>([0, 0]);
  useEffect(() => {
    const p = peaks.current;
    const next: [number, number] = [Math.max(pos(lv[0]), p[0] - 0.01), Math.max(pos(lv[1]), p[1] - 0.01)];
    peaks.current = next;
    setPk(next);
  }, [lv]);
  const black = playout?.black ?? false;
  const master = show?.show.audio.masterDb ?? 0;

  return (
    <aside className="flex w-[240px] shrink-0 flex-col border-l border-line2 bg-panel px-5 pt-5">
      <div className="flex items-baseline justify-between">
        <span className="mono text-[12px] font-bold tracking-[1.5px] text-sub">{t("master")}</span>
        <span className="text-[10px] text-dim">{show?.show.audio.device === "auto" ? "WASAPI default" : show?.show.audio.device}</span>
      </div>
      <div className="mt-4 flex flex-1 gap-3 pb-5">
        {[0, 1].map((i) => (
          <div key={i} className="flex flex-col items-center">
            <div className="relative h-[210px] w-[22px] border border-line2 bg-[#0b0c0d]">
              <div
                className="absolute inset-x-[2px] bottom-[1px]"
                style={{ height: `${pos(lv[i]) * 100}%`, background: "linear-gradient(to top,#2fb344 0%,#2fb344 72%,#f2a900 86%,#e5383b 100%)", backgroundSize: "100% 208px", backgroundPosition: "bottom" }}
              />
              <div className="absolute inset-x-[2px] h-[2px] bg-white" style={{ bottom: `${pk[i] * 100}%` }} />
            </div>
            <span className="mono mt-1 text-[11px] text-sub">{"LR"[i]}</span>
          </div>
        ))}
        <div className="relative h-[210px] w-8">
          {SCALE.map(([l, f]) => (
            <div key={l} className="absolute flex items-center gap-1" style={{ top: `${f * 100}%` }}>
              <div className="h-px w-1.5 bg-dim" />
              <span className="mono text-[10px] text-dim">{l}</span>
            </div>
          ))}
        </div>
        <div className="flex flex-col gap-2 pt-4">
          <span className="text-[11px] text-sub">{t("level")}</span>
          <span className="mono text-[24px] font-bold leading-none">{master.toFixed(1)}</span>
          <span className="mono text-[11px] text-sub">dB</span>
          <button className="mono mt-3 h-[34px] w-[70px] border border-line2 text-[12px] font-bold text-sub" onClick={() => send({ cmd: "toggleBlack" })}>
            <span className={black ? "text-sb" : ""}>{t("black")}</span>
          </button>
          <span className="mono text-center text-[10px] text-dim">B</span>
        </div>
      </div>
    </aside>
  );
}
