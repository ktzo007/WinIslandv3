import { useEffect, useMemo, useState } from "react";
import { call } from "../../shared/tauri";
import { baseName } from "../../shared/format";

/** Riwayat bermain dari log Rust (sessions.log), diagregasi per exe. */
export function useSessions(enabled) {
  const [raw, setRaw] = useState([]);
  useEffect(() => {
    if (enabled) call("game_sessions").then((r) => Array.isArray(r) && setRaw(r));
  }, [enabled]);
  return useMemo(() => {
    const by = {};
    for (const s of raw) {
      const k = baseName(s.exe).toLowerCase();
      const e = (by[k] ||= { exe: k, secs: 0, last: 0, n: 0 });
      e.secs += s.end - s.start;
      e.last = Math.max(e.last, s.end);
      e.n++;
    }
    return { byExe: by, recent: [...raw].reverse().slice(0, 6), totalSecs: Object.values(by).reduce((a, b) => a + b.secs, 0) };
  }, [raw]);
}

export const fmtHours = (secs) => (secs >= 3600 ? `${(secs / 3600).toFixed(secs >= 36000 ? 0 : 1)} j` : `${Math.max(1, Math.round(secs / 60))} mnt`);
