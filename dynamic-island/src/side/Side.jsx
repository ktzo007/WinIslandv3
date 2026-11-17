import { useEffect, useRef, useState } from "react";
import "../styles/side.css";
import { call, useTauriEvent } from "../shared/tauri";
import { useSettings } from "../shared/settings";
import { useTheme } from "../shared/themes";
import { Icon, IconButton } from "../shared/Icon";
import { useMedia } from "../island/hooks/useMedia";
import { useAudio } from "../island/hooks/useAudio";
import { usePower } from "../island/hooks/usePower";
import { useStats } from "../island/hooks/useStats";
import { useClock } from "../island/hooks/useClock";
import { useWeather } from "../island/hooks/useWeather";
import { useBoost } from "../island/hooks/useBoost";
import { useQuick } from "../island/hooks/useQuick";
import { useTodos } from "../island/hooks/useTodos";
import { Apps, GameSec, Home, Tasks, Tools } from "./sections";

const SEC = {
  home: { label: "Dasbor", icon: "home", C: Home },
  game: { label: "Game", icon: "gamepad", C: GameSec },
  apps: { label: "Aplikasi", icon: "grid", C: Apps },
  tasks: { label: "Tugas", icon: "checkSq", C: Tasks },
  tools: { label: "Alat", icon: "wrench", C: Tools },
};
const RAIL_W = 54;
const PANEL_W = 372;
const GAP = 8;

export default function Side() {
  const s = useSettings();
  useTheme();
  const sd = s.side;
  const secs = sd.sections.filter((k) => SEC[k]);
  const [visible, setVisible] = useState(true);
  const [hover, setHover] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [editing, setEditing] = useState(false);
  const [sec, setSec] = useState(secs[0] ?? "home");
  const [keepAwake, setKeepAwakeS] = useState(false);
  useTauriEvent("island-visibility", setVisible);

  const open = !sd.autohide || hover || pinned || editing;
  const cur = secs.includes(sec) ? sec : secs[0];
  const live = visible && open;

  // data (hanya aktif saat panel terbuka)
  const now = useClock(live, false);
  const media = useMedia(visible);
  const audio = useAudio(visible);
  const power = usePower(visible);
  const boost = useBoost(visible);
  const quick = useQuick(live && (cur === "home" || cur === "game"));
  const stats = useStats(live && cur === "home");
  const weather = useWeather(s.widgets.weatherOn, s.widgets.weatherCity, s.widgets.weatherUnit, live && cur === "home");
  const todos = useTodos();
  const setKeepAwake = (on) => { setKeepAwakeS(on); call("keep_awake", { on }); };
  const typing = {
    onFocus: () => { setEditing(true); call("island_focus", { on: true }); },
    onBlur: () => { setEditing(false); call("island_focus", { on: false }); },
  };

  // hover dengan jeda kecil
  const t = useRef(0);
  const enter = () => { clearTimeout(t.current); t.current = setTimeout(() => setHover(true), 90); };
  const leave = () => { clearTimeout(t.current); t.current = setTimeout(() => setHover(false), 360); };
  useEffect(() => () => clearTimeout(t.current), []);

  // area klik native: rail saja saat tertutup, rail+panel saat terbuka
  const H = Math.min(window.innerHeight - 24, 740);
  const RAIL_H = Math.min(H, 66 + secs.length * 50 + 94);
  const region = useRef({ w: 0, h: 0 });
  useEffect(() => {
    if (!visible) return;
    const w = open ? RAIL_W + GAP + PANEL_W : RAIL_W;
    const h = open ? H : RAIL_H;
    const prev = region.current;
    const apply = (a, b) => { region.current = { w: a, h: b }; call("set_island_shape", { w: a, h: b }); };
    let timer;
    if (w >= prev.w && h >= prev.h) apply(w, h);
    else {
      apply(Math.max(w, prev.w), Math.max(h, prev.h));
      timer = setTimeout(() => apply(w, h), 520);
    }
    return () => clearTimeout(timer);
  }, [open, visible, H, RAIL_H]);

  if (!visible) return null;
  const idx = Math.max(0, secs.indexOf(cur));
  const C = SEC[cur].C;
  const d = { s, now, media, audio, power, boost, quick, stats, weather, todos, typing, keepAwake, setKeepAwake };

  return (
    <div className="side-root" data-edge={sd.edge} data-open={open ? "1" : "0"} onMouseEnter={enter} onMouseLeave={leave}>
      <div className="side-wrap" style={{ "--H": `${H}px` }}>
        <nav className="rail">
          <div className="rail-in">
            <i className="rind" style={{ transform: `translateY(${idx * 50}px)` }} />
            {secs.map((k) => (
              <button key={k} type="button" className="rbtn" data-on={k === cur ? "1" : undefined} title={SEC[k].label} aria-label={SEC[k].label} onClick={() => setSec(k)}>
                <Icon name={SEC[k].icon} size={19} />
                {k === "game" && boost.active && <u />}
              </button>
            ))}
          </div>
          <div className="rail-foot">
            <IconButton name="pin" size={15} label={pinned ? "Lepas sematan" : "Sematkan terbuka"} active={pinned} className="sm" onClick={() => setPinned((p) => !p)} />
            <IconButton name="gear" size={15} label="Pengaturan" className="sm" onClick={() => call("open_settings_window")} />
          </div>
        </nav>
        <aside className="spanel" style={{ width: PANEL_W }}>
          <header className="shead">
            <Icon name={SEC[cur].icon} size={16} />
            <b>{SEC[cur].label}</b>
            <span className="grow" />
            {boost.active && <em className="live"><Icon name="rocket" size={11} /> Boost</em>}
          </header>
          <div className="sbody" key={cur}>
            <C d={d} />
          </div>
        </aside>
      </div>
    </div>
  );
}
