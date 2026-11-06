import { useEffect, useState } from "react";
import { useUI, useT } from "../store";
import { send, useEngine } from "../hooks";

function Mark() {
  // TRAPDOOR mark: stage floor (frame) with a trap door swung open (red).
  return (
    <svg viewBox="0 0 28 26" className="h-[26px] w-[28px]">
      <rect x="1" y="1" width="26" height="24" fill="none" stroke="#d9dcdf" strokeWidth="2" />
      <rect x="5" y="15" width="18" height="8" fill="#000" />
      <path d="M5,15 L23,15 L19,3 L9,3 Z" fill="#e5383b" />
    </svg>
  );
}

function Chip({ dot, label, value, active = true, onClick }: { dot: string; label: string; value: string; active?: boolean; onClick?: () => void }) {
  return (
    <button onClick={onClick} className="flex h-[30px] items-center gap-2 border border-line2 bg-panel2 px-3" title={value}>
      <span className="h-[9px] w-[9px] rounded-full" style={{ background: dot }} />
      <span className={`mono text-[11px] font-bold ${active ? "text-ink" : "text-dim"}`}>{label}</span>
      <span className={`mono max-w-[190px] truncate text-[11px] ${active ? "text-sub" : "text-dim"}`}>{value}</span>
    </button>
  );
}

export function HeaderBar() {
  const t = useT();
  const show = useUI((s) => s.show);
  const playout = useUI((s) => s.playout);
  const info = useUI((s) => s.info);
  const dirty = useUI((s) => s.dirty);
  const savedAt = useUI((s) => s.savedAt);
  const set = useUI((s) => s.set);
  const eng = useEngine();
  const [clock, setClock] = useState("");
  useEffect(() => {
    const f = () => setClock(new Date().toLocaleTimeString("en-GB", { hour12: false }));
    f();
    const id = setInterval(f, 250);
    return () => clearInterval(id);
  }, []);
  const locked = playout?.locked ?? false;
  const out = show?.show.output;
  const sim = (info?.output ?? "").startsWith("SIM");
  const dropped = playout?.droppedFrames ?? 0;

  return (
    <header className="flex h-14 shrink-0 items-center border-b border-line bg-panel px-5">
      <div className="flex items-center gap-3">
        <Mark />
        <span className="mono text-[19px] font-bold tracking-[3px]">TRAPDOOR</span>
      </div>
      <div className="mx-4 h-6 w-px bg-line2" />
      <div className="min-w-0">
        <div className="mono text-[10px] tracking-wider text-dim">{t("show")}</div>
        <div className="flex items-baseline gap-4">
          <span className="max-w-[300px] truncate text-[14px]">{show?.show.name ?? "—"}</span>
          <span className="text-[11px] text-sub">
            ● {dirty ? t("unsaved") : savedAt ? `${t("saved")} ${savedAt}` : ""}
          </span>
        </div>
      </div>
      <div className="ml-3 flex gap-1">
        {[
          ["new", () => eng?.newShow()],
          ["open", () => eng?.openShow()],
          [
            "save",
            async () => {
              const p = await eng?.saveShow();
              if (p) set({ dirty: false, savedAt: new Date().toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" }) });
            },
          ],
        ].map(([k, f]) => (
          <button key={k as string} onClick={f as () => void} disabled={locked} className="h-7 border border-line2 px-2 text-[12px] text-sub disabled:opacity-40">
            {t(k as string)}
          </button>
        ))}
      </div>
      <div className="flex-1 text-center">
        <span className="mono text-[26px] font-bold">{clock}</span>
      </div>
      <div className="flex items-center gap-2.5">
        {sim && <span className="mono text-[11px] font-bold text-sb">{t("simulator")}</span>}
        <Chip
          dot={sim ? "#5c6268" : "#2fb344"}
          label="OUT1"
          value={out ? `DISP${out.display || "-W"} ${out.width}×${out.height} ${show?.show.frameRate}` : "—"}
          onClick={() => !locked && set({ outputDialog: true })}
        />
        <Chip dot="#5c6268" label="NDI" value="—" active={false} />
        <Chip dot="#5c6268" label="SDI" value="—" active={false} />
        <Chip dot={dropped ? "#e5383b" : "#2fb344"} label="DROP" value={String(dropped)} />
        <button
          onClick={() => send({ cmd: "lock", on: !locked })}
          className={`h-[34px] w-[154px] border text-[13px] ${locked ? "border-sb bg-sb font-bold text-[#1a1200]" : "border-line2 text-sub"}`}
        >
          {locked ? t("locked") : `${t("lock")}  Ctrl+L`}
        </button>
      </div>
    </header>
  );
}
