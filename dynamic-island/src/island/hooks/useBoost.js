import { useCallback, useEffect, useState } from "react";
import { call, useTauriEvent } from "../../shared/tauri";

const OFF = { active: false, exe: "", since: 0, auto: false };

/** Status Game Boost (event dari Rust) + toggle manual. */
export function useBoost(visible = true) {
  const [b, setB] = useState(OFF);
  useTauriEvent("boost-state", setB);
  useEffect(() => {
    if (visible) call("boost_state").then((r) => r && setB(r));
  }, [visible]);
  const toggle = useCallback(async () => {
    const r = await call("boost_toggle");
    if (r) setB(r);
  }, []);
  return { ...b, toggle };
}
