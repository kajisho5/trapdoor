import { useEffect, useState } from "react";
import { useUI, useT, type Lang } from "../store";
import { useEngine } from "../hooks";
import type { DisplayInfo, FrameRate, OutputSettings, Preset } from "../types";

const MAX_W = 8192;
const MAX_H = 4320;
const RATES: FrameRate[] = ["23.976", "24", "25", "29.97", "30", "50", "59.94", "60"];

export function OutputDialog() {
  const t = useT();
  const open = useUI((s) => s.outputDialog);
  const set = useUI((s) => s.set);
  const show = useUI((s) => s.show);
  const lang = useUI((s) => s.lang);
  const setLang = useUI((s) => s.setLang);
  const eng = useEngine();
  const [presets, setPresets] = useState<Preset[]>([]);
  const [displays, setDisplays] = useState<DisplayInfo[]>([]);
  const [o, setO] = useState<OutputSettings | null>(null);
  const [rate, setRate] = useState<FrameRate>("59.94");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open || !eng || !show) return;
    eng.presets().then(setPresets);
    eng.displays().then(setDisplays);
    setO(structuredClone(show.show.output));
    setRate(show.show.frameRate);
  }, [open, eng, show]);

  if (!open || !o || !show) return null;
  const preset = presets.find((p) => p.width === o.width && p.height === o.height);
  const presetKey = preset ? `${preset.width}x${preset.height}` : "custom";
  const clampEven = (v: number, max: number, min: number) => Math.max(min, Math.min(max, Math.round(v))) & ~1;

  const apply = async () => {
    if (!eng) return;
    setBusy(true);
    try {
      if (rate !== show.show.frameRate) await eng.command({ cmd: "updateShow", settings: { ...show.show, frameRate: rate } });
      await eng.applyOutput({ ...o, width: clampEven(o.width, MAX_W, 320), height: clampEven(o.height, MAX_H, 180) });
      set({ outputDialog: false });
    } catch (e) {
      useUI.getState().flash(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={() => set({ outputDialog: false })}>
      <div className="w-[560px] border border-line2 bg-panel p-6" onClick={(e) => e.stopPropagation()}>
        <div className="mono mb-5 text-[13px] font-bold tracking-[1.5px] text-sub">{t("outputSettings").toUpperCase()}</div>
        <div className="grid grid-cols-2 gap-4">
          <label>
            <span className="lbl">{t("display")}</span>
            <select className="field" value={o.display} onChange={(e) => setO({ ...o, display: Number(e.target.value) })}>
              <option value={0}>{t("windowed")}</option>
              {displays.map((d) => (
                <option key={d.index} value={d.index}>
                  {d.index}: {d.width}×{d.height}
                  {d.primary ? " (primary)" : ""}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span className="lbl">Frame rate</span>
            <select className="field" value={rate} onChange={(e) => setRate(e.target.value as FrameRate)}>
              {RATES.map((r) => (
                <option key={r} value={r}>
                  {r}
                </option>
              ))}
            </select>
          </label>
          <label className="col-span-2">
            <span className="lbl">{t("resolution")}</span>
            <select
              className="field"
              value={presetKey}
              onChange={(e) => {
                if (e.target.value === "custom") return setO({ ...o, width: o.width + 2, height: o.height });
                const [w, h] = e.target.value.split("x").map(Number);
                setO({ ...o, width: w, height: h });
              }}
            >
              {presets.map((p) => (
                <option key={p.label} value={`${p.width}x${p.height}`}>
                  {p.label} — {p.width}×{p.height}
                </option>
              ))}
              <option value="custom">{t("custom")}…</option>
            </select>
          </label>
          <label>
            <span className="lbl">{t("width")}</span>
            <input className="field" type="number" min={320} max={MAX_W} step={2} value={o.width} onChange={(e) => setO({ ...o, width: Number(e.target.value) })} />
          </label>
          <label>
            <span className="lbl">{t("height")}</span>
            <input className="field" type="number" min={180} max={MAX_H} step={2} value={o.height} onChange={(e) => setO({ ...o, height: Number(e.target.value) })} />
          </label>
          <p className="col-span-2 text-[12px] text-sub">{t("maxRes")}</p>
          <label>
            <span className="lbl">{t("language")}</span>
            <select className="field" value={lang} onChange={(e) => setLang(e.target.value as Lang)}>
              <option value="en">English</option>
              <option value="ja">日本語</option>
            </select>
          </label>
        </div>
        <div className="mt-6 flex justify-end gap-3">
          <button className="h-9 border border-line2 px-5 text-[13px]" onClick={() => set({ outputDialog: false })}>
            {t("cancel")}
          </button>
          <button disabled={busy} className="h-9 border border-sel bg-sel px-5 text-[13px] font-bold text-[#0e0f10]" onClick={apply}>
            {t("apply")}
          </button>
        </div>
      </div>
    </div>
  );
}
