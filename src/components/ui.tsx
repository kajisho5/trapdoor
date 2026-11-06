import { useEffect, useRef } from "react";
import type { EndAction, Media } from "../types";
import { useEngine } from "../hooks";

/** Demo placeholders (fictional art) for the simulator. */
const DEMO: Record<string, [string, string]> = {
  count: ["#1b2a4a", "#0d1424"],
  open: ["#5b2a86", "#e5484d"],
  bgm: ["#0f3d3e", "#30a46c"],
  guest: ["#3a2f12", "#f5a524"],
  award: ["#4a1d2c", "#ffb3c1"],
  break: ["#12304a", "#3e9bff"],
  end: ["#2b2b2b", "#d0d0d0"],
  logo: ["#0b0d10", "#3e9bff"],
  sponsor: ["#33240f", "#ffd27a"],
};

function DemoArt({ look }: { look: string }) {
  const [a, b] = DEMO[look] ?? ["#222", "#444"];
  return (
    <svg viewBox="0 0 160 90" preserveAspectRatio="none" className="block h-full w-full">
      <defs>
        <linearGradient id={`g-${look}`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor={a} />
          <stop offset="1" stopColor={b} />
        </linearGradient>
      </defs>
      <rect width="160" height="90" fill={`url(#g-${look})`} />
      {look === "count" && (
        <text x="80" y="62" textAnchor="middle" fontSize="44" fontFamily="IBM Plex Mono" fontWeight="700" fill="#fff" opacity=".85">
          10
        </text>
      )}
      {(look === "open" || look === "award") && (
        <>
          <circle cx="112" cy="40" r="25" fill="#fff" opacity=".18" />
          <rect x="16" y="56" width="72" height="7" fill="#fff" opacity=".6" />
          <rect x="16" y="68" width="48" height="5" fill="#fff" opacity=".35" />
        </>
      )}
      {look === "bgm" &&
        Array.from({ length: 9 }, (_, i) => {
          const h = 90 * (0.15 + (0.5 * Math.abs(((i * 37) % 10) - 5)) / 5);
          return <rect key={i} x={19 + i * 13.6} y={72 - h} width={8} height={h} fill="#fff" opacity=".5" />;
        })}
      {(look === "guest" || look === "sponsor") && <path d="M16,76 L64,31 L96,58 L120,40 L152,76 Z" fill="#fff" opacity=".25" />}
      {(look === "break" || look === "logo") && (
        <>
          <circle cx="80" cy="45" r="23" fill="none" stroke="#fff" strokeWidth="2.7" opacity=".7" />
          <path d="M72,33 L93,45 L72,57 Z" fill="#fff" opacity=".7" />
        </>
      )}
      {look === "end" && (
        <>
          <rect x="40" y="27" width="80" height="5" fill="#fff" opacity=".7" />
          {[0, 1, 2].map((i) => (
            <rect key={i} x="48" y={43 + i * 11} width="64" height="3.6" fill="#fff" opacity=".35" />
          ))}
        </>
      )}
    </svg>
  );
}

function Bars() {
  const cols = ["#c0c0c0", "#c0c000", "#00c0c0", "#00c000", "#c000c0", "#c00000", "#0000c0"];
  return (
    <div className="flex h-full w-full flex-col">
      <div className="flex flex-[7]">
        {cols.map((c) => (
          <div key={c} className="flex-1" style={{ background: c }} />
        ))}
      </div>
      <div className="flex-[3] bg-[#101010]" />
    </div>
  );
}

export function Thumb({ media, className = "", strip, second, totalSeconds }: { media?: Media | null; className?: string; strip?: boolean; second?: number; totalSeconds?: number }) {
  const eng = useEngine();
  const t = media?.thumb;
  let inner: React.ReactNode = <div className="h-full w-full bg-[#0b0c0d]" />;
  if (media?.kind === "audio") {
    inner = <div className="flex h-full w-full items-center justify-center bg-[#0b0c0d] text-[11px] text-dim mono">AUDIO</div>;
  }
  if (t?.startsWith("demo:")) {
    const look = t.slice(5);
    inner = look === "bars" ? <Bars /> : look === "black" ? <div className="h-full w-full bg-black" /> : <DemoArt look={look} />;
  } else if (strip && media?.filmstrip && eng) {
    const n = Math.min(600, Math.max(1, Math.ceil(totalSeconds ?? 1)));
    const idx = Math.min(n - 1, Math.max(0, Math.floor(second ?? 0)));
    inner = (
      <div
        className="h-full w-full bg-black"
        style={{
          backgroundImage: `url("${eng.fileUrl(media.filmstrip)}")`,
          backgroundSize: `${n * 100}% 100%`,
          backgroundPosition: `${n > 1 ? (idx / (n - 1)) * 100 : 0}% 0`,
        }}
      />
    );
  } else if (t && eng) {
    inner = <img src={eng.fileUrl(t) ?? ""} className="block h-full w-full object-cover" draggable={false} />;
  }
  return <div className={`overflow-hidden ${className}`}>{inner}</div>;
}

/** Waveform from 0..255 peaks. `played` 0..1 drawn in `playedColor`. */
export function Wave({
  peaks,
  from = 0,
  to,
  played = 0,
  color = "#3a3d41",
  playedColor = "#e5383b",
  className = "",
  bar = 3,
}: {
  peaks?: number[];
  from?: number;
  to?: number;
  played?: number;
  color?: string;
  playedColor?: string;
  className?: string;
  bar?: number;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const draw = () => {
      const dpr = window.devicePixelRatio || 1;
      const w = c.clientWidth;
      const h = c.clientHeight;
      if (!w || !h) return;
      c.width = Math.round(w * dpr);
      c.height = Math.round(h * dpr);
      const g = c.getContext("2d")!;
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      g.clearRect(0, 0, w, h);
      if (!peaks || peaks.length === 0) return;
      const a = Math.max(0, Math.min(peaks.length, from));
      const b = Math.max(a + 1, Math.min(peaks.length, to ?? peaks.length));
      const n = Math.floor(w / bar);
      const mid = h / 2;
      for (let i = 0; i < n; i++) {
        const s = a + Math.floor(((b - a) * i) / n);
        const e = Math.max(s + 1, a + Math.floor(((b - a) * (i + 1)) / n));
        let m = 0;
        for (let k = s; k < e; k++) m = Math.max(m, peaks[k] ?? 0);
        const amp = Math.max(0.6, (m / 255) * (h / 2));
        g.fillStyle = i / n < played ? playedColor : color;
        g.fillRect(i * bar, mid - amp, Math.max(1, bar - 1), amp * 2);
      }
    };
    draw();
    const ro = new ResizeObserver(draw);
    ro.observe(c);
    return () => ro.disconnect();
  }, [peaks, from, to, played, color, playedColor, bar]);
  return <canvas ref={ref} className={`block ${className}`} />;
}

export function EndIcon({ end, className = "" }: { end: EndAction; className?: string }) {
  const d: Record<EndAction, string> = {
    loop: "M-6,-2 A6,6 0 1 1 -2,5 M-6,-2 L-8,-7 M-6,-2 L-1,-3",
    next: "M-7,0 L5,0 M1,-5 L6,0 L1,5",
    hold: "M-5,-6 L-5,6 M5,-6 L5,6",
    black: "M-6,-6 L6,-6 L6,6 L-6,6 Z",
    goto: "M-7,4 L-7,-2 L5,-2 M1,-7 L6,-2 L1,3",
  };
  const blue = end === "loop" || end === "next" || end === "goto";
  return (
    <svg viewBox="-9 -9 18 18" className={`h-4 w-4 ${className}`}>
      <path d={d[end]} fill="none" stroke={blue ? "#d0d5da" : "#8a9096"} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function Keycap({ k, dark }: { k: string; dark?: boolean }) {
  return (
    <span className={`absolute right-1.5 top-1.5 px-1 text-[10px] leading-4 mono ${dark ? "bg-black/35 text-[#04140b]" : "bg-black/35 text-sub"}`}>{k}</span>
  );
}
