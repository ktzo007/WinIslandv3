import { useEffect, useMemo, useState } from "react";
import { call } from "../../shared/tauri";

/**
 * Metrik gaming: ping, GPU NVIDIA, kecepatan jaringan, CPU/RAM. Dipoll tiap 2 dtk HANYA selama
 * `enabled` (panel Game terbuka). Nonaktif → timer dibersihkan, nol aktivitas.
 */
export function useGameMetrics(enabled, host = "1.1.1.1") {
  const [m, setM] = useState({ ping: [], gpu: null, net: { downBps: 0, upBps: 0 }, cpu: null, ram: null, vram: null });
  useEffect(() => {
    if (!enabled) return;
    let alive = true;
    let timer;
    const tick = async () => {
      const [p, g, n, s] = await Promise.all([call("ping", { host }), call("gpu_info"), call("net_speed"), call("get_stats")]);
      if (!alive) return;
      setM((prev) => ({
        ping: [...prev.ping, p].slice(-30),
        gpu: g,
        net: n || prev.net,
        cpu: s?.cpu ?? prev.cpu,
        ram: s?.ramPercent ?? prev.ram,
      }));
      timer = setTimeout(tick, 2000);
    };
    tick();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [enabled, host]);

  return useMemo(() => {
    const ok = m.ping.filter((x) => x != null);
    const recent = m.ping.slice(-20);
    const loss = recent.length ? Math.round((recent.filter((x) => x == null).length / recent.length) * 100) : 0;
    let jitter = 0;
    for (let i = 1; i < ok.length; i++) jitter += Math.abs(ok[i] - ok[i - 1]);
    jitter = ok.length > 1 ? Math.round(jitter / (ok.length - 1)) : 0;
    return { ...m, cur: m.ping.at(-1) ?? null, hasPing: m.ping.length > 0, loss, jitter };
  }, [m]);
}

export const fmtRate = (bps) => {
  const b = bps * 8;
  if (b >= 1e9) return `${(b / 1e9).toFixed(1)} Gb/s`;
  if (b >= 1e6) return `${(b / 1e6).toFixed(b >= 1e7 ? 0 : 1)} Mb/s`;
  if (b >= 1e3) return `${Math.round(b / 1e3)} Kb/s`;
  return `${Math.round(b)} b/s`;
};
