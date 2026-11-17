import { useCallback, useEffect, useState } from "react";
import { call } from "../../shared/tauri";

export const PLANS = { saver: "a1841308-3541-4fab-bc81-f71556f20b4a", balanced: "381b4222-f694-41f0-9685-ff5bb260df2e", high: "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c" };

/** Status cepat sistem: Jangan Ganggu, Windows Game Mode, power plan aktif. Dimuat saat `enabled`. */
export function useQuick(enabled) {
  const [q, setQ] = useState({ dnd: false, gameMode: true, power: null });
  const refresh = useCallback(async () => {
    const r = await call("quick_state");
    if (r) setQ(r);
  }, []);
  useEffect(() => {
    if (enabled) refresh();
  }, [enabled, refresh]);
  const toggleDnd = useCallback(async () => {
    const next = !q.dnd;
    setQ((p) => ({ ...p, dnd: next }));
    await call("dnd_set", { on: next });
  }, [q.dnd]);
  const toggleGameMode = useCallback(async () => {
    const next = !q.gameMode;
    setQ((p) => ({ ...p, gameMode: next }));
    await call("game_mode_set", { on: next });
  }, [q.gameMode]);
  const setPlan = useCallback(
    async (key) => {
      setQ((p) => ({ ...p, power: PLANS[key] }));
      await call("power_set", { guid: PLANS[key] });
      refresh();
    },
    [refresh]
  );
  const plan = q.power === PLANS.high ? "high" : q.power === PLANS.saver ? "saver" : q.power === PLANS.balanced ? "balanced" : null;
  return { ...q, plan, toggleDnd, toggleGameMode, setPlan, refresh };
}
