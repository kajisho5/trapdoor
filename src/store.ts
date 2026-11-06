import { create } from "zustand";
import type { EngineInfo, LogEntry, PlayoutState, Show } from "./types";
import en from "./i18n/en.json";
import ja from "./i18n/ja.json";

export type Lang = "en" | "ja";
export type Tab = "bank" | "inspector" | "media" | "log";

const DICTS: Record<Lang, Record<string, string>> = { en, ja };

function loadLang(): Lang {
  try {
    const v = localStorage.getItem("trapdoor.lang");
    return v === "ja" ? "ja" : "en";
  } catch {
    return "en";
  }
}

interface UIState {
  show: Show | null;
  playout: PlayoutState | null;
  info: EngineInfo | null;
  logs: LogEntry[];
  selectedCueId: string | null;
  tab: Tab;
  lang: Lang;
  dirty: boolean;
  savedAt: string | null;
  outputDialog: boolean;
  toast: string | null;
  setShow(s: Show): void;
  setPlayout(p: PlayoutState): void;
  addLog(l: LogEntry): void;
  select(id: string | null): void;
  setTab(t: Tab): void;
  setLang(l: Lang): void;
  set(p: Partial<UIState>): void;
  flash(msg: string): void;
}

export const useUI = create<UIState>((set, get) => ({
  show: null,
  playout: null,
  info: null,
  logs: [],
  selectedCueId: null,
  tab: "bank",
  lang: loadLang(),
  dirty: false,
  savedAt: null,
  outputDialog: false,
  toast: null,
  setShow: (s) => set((st) => ({ show: s, dirty: st.show !== null })),
  setPlayout: (p) => set({ playout: p }),
  addLog: (l) => set((st) => ({ logs: [...st.logs.slice(-499), l] })),
  select: (id) => set({ selectedCueId: id }),
  setTab: (t) => set({ tab: t }),
  setLang: (l) => {
    try {
      localStorage.setItem("trapdoor.lang", l);
    } catch {
      /* storage may be unavailable */
    }
    set({ lang: l });
  },
  set: (p) => set(p),
  flash: (msg) => {
    set({ toast: msg });
    setTimeout(() => {
      if (get().toast === msg) set({ toast: null });
    }, 2500);
  },
}));

export function useT() {
  const lang = useUI((s) => s.lang);
  return (key: string, vars?: Record<string, string | number>) => {
    let s = DICTS[lang][key] ?? DICTS.en[key] ?? key;
    if (vars) for (const [k, v] of Object.entries(vars)) s = s.replace(`{${k}}`, String(v));
    return s;
  };
}
