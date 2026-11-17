import { Icon, IconButton } from "../shared/Icon";
import { FileIcon, Slider } from "../shared/ui";

// ───────── kontrol dasar ─────────
export const Row = ({ title, desc, children }) => (
  <div className="row">
    <div className="rt">
      <b>{title}</b>
      {desc && <span>{desc}</span>}
    </div>
    <div className="rc">{children}</div>
  </div>
);
export const Group = ({ title, children }) => (
  <section className="group-wrap">
    {title && <h3>{title}</h3>}
    <div className="group">{children}</div>
  </section>
);
export const Toggle = ({ on, onChange, label }) => (
  <button type="button" role="switch" aria-checked={!!on} aria-label={label} className="tg" data-on={on ? "1" : undefined} onClick={() => onChange(!on)}>
    <i />
  </button>
);
export const Seg = ({ value, options, onChange }) => (
  <div className="sseg">
    {options.map(([v, n]) => (
      <button key={String(v)} type="button" data-on={value === v ? "1" : undefined} onClick={() => onChange(v)}>
        {n}
      </button>
    ))}
  </div>
);
export function Range({ value, min, max, step = 1, onChange, fmt = (v) => v }) {
  return (
    <div className="range">
      <Slider value={(value - min) / (max - min)} onChange={(r) => onChange(Math.round((min + r * (max - min)) / step) * step)} label="" />
      <span className="num">{fmt(value)}</span>
    </div>
  );
}

export async function pickApp() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({ multiple: false, filters: [{ name: "Aplikasi & pintasan", extensions: ["exe", "lnk", "bat", "url"] }] });
    return typeof p === "string" ? p : null;
  } catch {
    return null;
  }
}

/** Editor daftar aplikasi (dock & pintasan): nama, urut, hapus. */
export function AppList({ items, onChange, empty }) {
  const set = (i, patch) => onChange(items.map((it, k) => (k === i ? { ...it, ...patch } : it)));
  const move = (i, d) => {
    const j = i + d;
    if (j < 0 || j >= items.length) return;
    const next = [...items];
    [next[i], next[j]] = [next[j], next[i]];
    onChange(next);
  };
  if (!items.length) return <div className="aempty">{empty}</div>;
  return (
    <div className="alist2">
      {items.map((it, i) => (
        <div key={it.path} className="arow">
          {/^https?:/i.test(it.path) ? <span className="gl"><Icon name="globe" size={18} /></span> : <FileIcon path={it.path} size={30} radius={8} />}
          <div className="ainfo">
            <input value={it.name} onChange={(e) => set(i, { name: e.target.value })} aria-label="Nama" />
            <span title={it.path}>{it.path}</span>
          </div>
          <IconButton name="chevU" size={14} label="Naik" className="sm" disabled={i === 0} onClick={() => move(i, -1)} />
          <IconButton name="chevD" size={14} label="Turun" className="sm" disabled={i === items.length - 1} onClick={() => move(i, 1)} />
          <IconButton name="trash" size={14} label="Hapus" className="sm" onClick={() => onChange(items.filter((_, k) => k !== i))} />
        </div>
      ))}
    </div>
  );
}

