import { useEffect } from "react";
import { HeaderBar } from "./components/HeaderBar";
import { CueList } from "./components/CueList";
import { PlayoutPanel } from "./components/PlayoutPanel";
import { Bottom } from "./components/Bottom";
import { Meters } from "./components/Meters";
import { OutputDialog } from "./components/OutputDialog";
import { engine } from "./lib/ipc";
import { send } from "./hooks";
import { useUI } from "./store";

function useEngineEvents() {
  useEffect(() => {
    let off: (() => void)[] = [];
    let alive = true;
    (async () => {
      const e = await engine();
      const ui = useUI.getState();
      const [show, info] = await Promise.all([e.getShow(), e.info()]);
      if (!alive) return;
      useUI.setState({ show, info, dirty: false });
      off = await Promise.all([
        e.onPlayout((p) => ui.setPlayout(p)),
        e.onShow((s) => useUI.getState().setShow(s)),
        e.onLog((l) => useUI.getState().addLog(l)),
        e.onDrop(async (paths) => {
          if (useUI.getState().playout?.locked) return;
          await e.addMedia(paths, useUI.getState().selectedCueId);
        }),
      ]);
      // engine info can change after the output is rebuilt
      const id = setInterval(async () => useUI.setState({ info: await e.info() }), 3000);
      off.push(() => clearInterval(id));
    })();
    return () => {
      alive = false;
      off.forEach((f) => f());
    };
  }, []);
}

/** Keyboard map — docs/design.md §4.4 */
function useKeys() {
  useEffect(() => {
    let lastEsc = 0;
    let stopTimer: number | null = null;
    const typing = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement;
      return el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT");
    };
    const down = async (e: KeyboardEvent) => {
      const ui = useUI.getState();
      if (e.ctrlKey && e.key.toLowerCase() === "l") {
        e.preventDefault();
        send({ cmd: "lock", on: !ui.playout?.locked });
        return;
      }
      if (e.ctrlKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        const p = await (await engine()).saveShow();
        if (p) useUI.setState({ dirty: false, savedAt: new Date().toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" }) });
        return;
      }
      if (e.ctrlKey && e.key.toLowerCase() === "o") {
        e.preventDefault();
        (await engine()).openShow();
        return;
      }
      if (/^F([1-9]|1[0-2])$/.test(e.key)) {
        e.preventDefault();
        const n = Number(e.key.slice(1));
        send({ cmd: "bank", slot: e.shiftKey ? `S-F${n}` : `F${n}` });
        return;
      }
      if (typing(e) || e.repeat) {
        if (e.key === "Escape") (e.target as HTMLElement).blur();
        return;
      }
      switch (e.key) {
        case " ":
          e.preventDefault();
          send({ cmd: "go" });
          break;
        case "Enter":
          if (ui.selectedCueId) send({ cmd: "take", cueId: ui.selectedCueId });
          break;
        case "ArrowDown":
          e.preventDefault();
          send({ cmd: "standbyNext" });
          break;
        case "ArrowUp":
          e.preventDefault();
          send({ cmd: "standbyPrev" });
          break;
        case "p":
        case "P":
          send({ cmd: "togglePause" });
          break;
        case "Escape": {
          const now = performance.now();
          if (now - lastEsc < 500) {
            send({ cmd: "panic" });
            lastEsc = 0;
          } else lastEsc = now;
          break;
        }
        case "s":
        case "S":
          stopTimer = window.setTimeout(() => send({ cmd: "stop" }), 500);
          break;
        case "1":
        case "2":
        case "3":
          send({ cmd: "gotoRemain", seconds: Number(e.key) * 10 });
          break;
        case "l":
        case "L":
          send({ cmd: "toggleLoop" });
          break;
        case "b":
        case "B":
          send({ cmd: "toggleBlack" });
          break;
      }
    };
    const up = (e: KeyboardEvent) => {
      if ((e.key === "s" || e.key === "S") && stopTimer) {
        clearTimeout(stopTimer);
        stopTimer = null;
      }
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, []);
}

export default function App() {
  useEngineEvents();
  useKeys();
  const toast = useUI((s) => s.toast);
  const ready = useUI((s) => s.show !== null);
  return (
    <div className="flex h-full min-h-[900px] min-w-[1440px] flex-col">
      <HeaderBar />
      <div className="flex min-h-0 flex-1">
        <CueList />
        <PlayoutPanel />
      </div>
      <div className="flex h-[320px] shrink-0 border-t border-line2 bg-panel">
        <Bottom />
        <Meters />
      </div>
      <OutputDialog />
      {toast && <div className="fixed bottom-6 left-1/2 z-50 -translate-x-1/2 border border-sb bg-panel2 px-5 py-2 text-[13px] text-sb">{toast}</div>}
      {!ready && <div className="fixed inset-0 flex items-center justify-center mono text-sub">TRAPDOOR</div>}
    </div>
  );
}
