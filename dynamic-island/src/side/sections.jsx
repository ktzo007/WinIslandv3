import { useEffect, useMemo, useState } from "react";
import { Icon, IconButton } from "../shared/Icon";
import { BoostButton, FileIcon, Ring, Slider, Sparkline } from "../shared/ui";
import { call } from "../shared/tauri";
import { baseName, clamp, fmtClock, fmtTrack, stem } from "../shared/format";
import { updateSettings } from "../shared/settings";
import { WMO } from "../island/hooks/useWeather";
import { fmtRate, useGameMetrics } from "../island/hooks/useGameMetrics";
import { fmtHours, useSessions } from "../island/hooks/useSessions";
import { pingTone } from "../island/panels/GamePanel";

const DAYS = ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"];
const MONTHS = ["Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November", "Desember"];

const Sec = ({ title, action, children }) => (
  <section className="ssec">
    <header>
      <b>{title}</b>
      {action}
    </header>
    {children}
  </section>
);

function QTile({ icon, label, on, onClick }) {
  return (
    <button type="button" className="qtile" data-on={on ? "1" : undefined} onClick={onClick}>
      <span><Icon name={icon} size={18} /></span>
      <em>{label}</em>
    </button>
  );
}

// ───────────────────────── Dasbor ─────────────────────────
export function Home({ d }) {
  const { now, s, media, audio, stats, power, boost, quick, weather, keepAwake, setKeepAwake } = d;
  const info = media.info;
  const cur = stats.cur;
  const w = weather.data;
  const drift = info?.playing && info.updatedMs > 0 ? Date.now() - info.updatedMs : 0;
  const pos = info?.durationMs ? clamp(info.positionMs + drift, 0, info.durationMs) : 0;
  return (
    <>
      <div className="hero">
        <div>
          <div className="hclock num">{fmtClock(now, s.island.clock24)}</div>
          <div className="hdate">{DAYS[now.getDay()]}, {now.getDate()} {MONTHS[now.getMonth()]}</div>
        </div>
        {s.widgets.weatherOn && w && (
          <div className="hwx">
            <span>{(WMO[w.code] || ["🌡️"])[0]}</span>
            <b className="num">{w.temp}°</b>
            <em>{w.place}</em>
          </div>
        )}
      </div>

      <Sec title="Sedang diputar">
        {info ? (
          <div className="mmini">
            <div className="mart">{media.art ? <img src={media.art} alt="" draggable={false} /> : <Icon name="music" size={20} />}</div>
            <div className="mmeta">
              <b>{info.title}</b>
              <span>{info.artist || info.album}</span>
              <div className="mprog"><i style={{ transform: `scaleX(${info.durationMs ? pos / info.durationMs : 0})` }} /></div>
            </div>
            <div className="mctl">
              <IconButton name="prev" size={15} label="Sebelumnya" className="sm" onClick={() => media.control("prev")} />
              <IconButton name={info.playing ? "pause" : "play"} size={16} label="Putar/jeda" className="md solid" onClick={() => media.control("play_pause")} />
              <IconButton name="next" size={15} label="Berikutnya" className="sm" onClick={() => media.control("next")} />
            </div>
          </div>
        ) : (
          <div className="sempty">Tidak ada musik diputar</div>
        )}
        <div className="vrow2">
          <IconButton name={audio.muted || audio.volume === 0 ? "mute" : "volume"} size={15} label="Bisukan" className="sm" onClick={audio.toggleMute} />
          <Slider value={audio.muted ? 0 : audio.volume} onChange={audio.setVolume} label="Volume" className="wide" />
          <span className="num vp">{audio.muted ? "–" : Math.round(audio.volume * 100)}</span>
        </div>
      </Sec>

      <Sec title="Aksi cepat">
        <div className="qgrid">
          <QTile icon="rocket" label="Boost" on={boost.active} onClick={boost.toggle} />
          <QTile icon="bellOff" label="Senyap" on={quick.dnd} onClick={quick.toggleDnd} />
          <QTile icon="coffee" label="Terjaga" on={keepAwake} onClick={() => setKeepAwake(!keepAwake)} />
          <QTile icon="scissors" label="Tangkap" onClick={() => call("sys_action", { name: "screenshot" })} />
          <QTile icon="lock" label="Kunci" onClick={() => call("sys_action", { name: "lock" })} />
          <QTile icon="activity" label="Task Mgr" onClick={() => call("sys_action", { name: "taskmgr" })} />
          <QTile icon="eyeOff" label="Sembunyi" onClick={() => call("set_manual_hidden", { hidden: true })} />
          <QTile icon="gear" label="Pengaturan" onClick={() => call("open_settings_window")} />
        </div>
      </Sec>

      <Sec title="Sistem">
        <div className="rings">
          {[
            ["CPU", cur?.cpu != null ? Math.round(cur.cpu) : null, "%"],
            ["RAM", cur?.ramPercent ?? null, "%"],
            ["Disk", cur?.diskPercent ?? null, "%"],
            [power.hasBattery ? (power.charging ? "Mengisi" : "Baterai") : "Daya", power.hasBattery ? power.percent : null, power.hasBattery ? "%" : ""],
          ].map(([l, v, u]) => (
            <div key={l} className="rg">
              <Ring value={(v ?? 0) / 100} size={58} stroke={5}>
                <span className="num">{v ?? (l === "Daya" ? "AC" : "–")}<small>{v != null ? u : ""}</small></span>
              </Ring>
              <em>{l}</em>
            </div>
          ))}
        </div>
      </Sec>
    </>
  );
}

// ───────────────────────── Game ─────────────────────────
export function GameSec({ d }) {
  const { s, boost, quick } = d;
  const host = s.game.pingHost || "1.1.1.1";
  const g = useGameMetrics(true, host);
  const sess = useSessions(true);
  const gpu = g.gpu;
  const chips = [
    s.game.power && "Performa maks.",
    s.game.dnd && "Senyap",
    s.game.gameMode && "Game Mode",
    s.game.priority && "Prioritas tinggi",
    s.game.cleanRam && "Bersihkan RAM",
    s.game.close.length > 0 && `Tutup ${s.game.close.length} app`,
  ].filter(Boolean);
  return (
    <>
      <BoostButton boost={boost} />
      <div className="chips">
        {chips.map((c) => <span key={c}>{c}</span>)}
        <button type="button" className="chipedit" onClick={() => call("open_settings_window")}>Atur profil</button>
      </div>
      <div className="auto">
        <Icon name="sparkle" size={13} /> Boost otomatis saat game fullscreen: <b>{s.game.auto ? "aktif" : "mati"}</b>
      </div>

      <Sec title="Power plan">
        <div className="sseg2">
          {[["saver", "Hemat"], ["balanced", "Seimbang"], ["high", "Performa"]].map(([k, n]) => (
            <button key={k} type="button" data-on={quick.plan === k ? "1" : undefined} onClick={() => quick.setPlan(k)}>{n}</button>
          ))}
        </div>
      </Sec>

      <Sec title="Live">
        <div className="mgrid">
          <div className="mc">
            <em>PING</em>
            <b className={`num ${pingTone(g.cur)}`}>{!g.hasPing ? "–" : g.cur == null ? "timeout" : `${g.cur} ms`}</b>
            <span className="num">jitter {g.jitter} · loss {g.loss}%</span>
            <Sparkline data={g.ping.map((v) => v ?? 150)} max={150} />
          </div>
          <div className="mc">
            <em>{gpu ? "GPU" : "CPU"}</em>
            <b className="num">{gpu ? `${gpu.util}%` : g.cpu != null ? `${Math.round(g.cpu)}%` : "–"}</b>
            <span className="num">{gpu ? `${gpu.temp}°C · ${(gpu.memUsed / 1024).toFixed(1)}/${(gpu.memTotal / 1024).toFixed(0)} GB` : "GPU: hanya NVIDIA"}</span>
            <div className="gbar"><i style={{ transform: `scaleX(${(gpu ? gpu.util : g.cpu ?? 0) / 100})` }} /></div>
          </div>
          <div className="mc">
            <em>JARINGAN</em>
            <b className="num"><Icon name="down" size={12} /> {fmtRate(g.net.downBps)}</b>
            <b className="num"><Icon name="up" size={12} /> {fmtRate(g.net.upBps)}</b>
          </div>
          <div className="mc">
            <em>RAM</em>
            <b className="num">{g.ram ?? "–"}%</b>
            <div className="gbar"><i style={{ transform: `scaleX(${(g.ram ?? 0) / 100})` }} /></div>
          </div>
        </div>
      </Sec>

      <Sec title="Library game" action={<button type="button" className="mini-act" onClick={() => call("open_settings_window")}>Kelola</button>}>
        {s.game.library.length === 0 ? (
          <div className="sempty">Tambahkan game di Pengaturan → Game. Waktu bermain tercatat otomatis.</div>
        ) : (
          <div className="lib">
            {s.game.library.map((x) => {
              const k = baseName(x.path).toLowerCase();
              const t = sess.byExe[k];
              return (
                <button key={x.path} type="button" className="gcard" onClick={() => call("launch", { path: x.path })} title={x.path}>
                  <FileIcon path={x.path} size={44} radius={12} />
                  <b>{x.name || stem(x.path)}</b>
                  <span>{t ? fmtHours(t.secs) : "Belum dimainkan"}</span>
                </button>
              );
            })}
          </div>
        )}
      </Sec>

      <Sec title="Riwayat bermain">
        {sess.recent.length === 0 ? (
          <div className="sempty">Belum ada sesi (≥ 1 menit) yang tercatat.</div>
        ) : (
          sess.recent.map((r, i) => (
            <div key={i} className="srow2">
              <Icon name="history" size={13} />
              <b>{stem(r.exe)}</b>
              <span className="num">{fmtHours(r.end - r.start)} · {new Date(r.start * 1000).toLocaleDateString("id-ID", { day: "numeric", month: "short" })}</span>
            </div>
          ))
        )}
        {sess.totalSecs > 0 && <div className="total num">Total bermain: {fmtHours(sess.totalSecs)}</div>}
      </Sec>
    </>
  );
}

// ───────────────────────── Aplikasi ─────────────────────────
export function Apps({ d }) {
  const { s, typing } = d;
  const [q, setQ] = useState("");
  const [wins, setWins] = useState([]);
  useEffect(() => {
    call("list_windows").then((r) => Array.isArray(r) && setWins(r));
  }, []);
  const items = useMemo(() => {
    const seen = new Set();
    return [...s.dock.pinned, ...s.shortcuts, ...s.game.library].filter((x) => !seen.has(x.path) && seen.add(x.path));
  }, [s.dock.pinned, s.shortcuts, s.game.library]);
  const shown = items.filter((x) => (x.name || stem(x.path)).toLowerCase().includes(q.toLowerCase()));
  return (
    <>
      <div className="search">
        <Icon name="search" size={14} />
        <input placeholder="Cari aplikasi…" value={q} onChange={(e) => setQ(e.target.value)} {...typing} />
      </div>
      <Sec title="Peluncur">
        {shown.length === 0 ? (
          <div className="sempty">{items.length ? "Tidak ditemukan" : "Tambahkan aplikasi di Pengaturan → Dock / Pintasan / Game."}</div>
        ) : (
          <div className="agrid">
            {shown.map((x) => (
              <button key={x.path} type="button" className="acard" onClick={() => call("launch", { path: x.path })} title={x.path}>
                {/^https?:/i.test(x.path) ? <span className="gl"><Icon name="globe" size={22} /></span> : <FileIcon path={x.path} size={42} radius={11} />}
                <span>{x.name || stem(x.path)}</span>
              </button>
            ))}
          </div>
        )}
      </Sec>
      <Sec title="Jendela terbuka" action={<button type="button" className="mini-act" onClick={() => call("list_windows").then((r) => Array.isArray(r) && setWins(r))}>Segarkan</button>}>
        {wins.slice(0, 10).map((w) => (
          <div key={w.hwnd} className="wrow">
            <FileIcon path={w.exe} size={22} radius={6} />
            <button type="button" className="wt" onClick={() => call("activate_window", { hwnd: w.hwnd })} title={w.title}>{w.title}</button>
            <IconButton name="x" size={12} label="Tutup jendela" className="xs" onClick={() => { call("close_window", { hwnd: w.hwnd }); setWins((p) => p.filter((x) => x.hwnd !== w.hwnd)); }} />
          </div>
        ))}
        {wins.length === 0 && <div className="sempty">Tidak ada jendela.</div>}
      </Sec>
    </>
  );
}

// ───────────────────────── Tugas ─────────────────────────
export function Tasks({ d }) {
  const { todos, typing } = d;
  const [text, setText] = useState("");
  const left = todos.todos.filter((t) => !t.done).length;
  return (
    <>
      <Sec title={`To-do · ${left} tersisa`} action={todos.todos.some((t) => t.done) && <button type="button" className="mini-act" onClick={todos.clearDone}>Hapus selesai</button>}>
        <form className="addtodo" onSubmit={(e) => { e.preventDefault(); todos.add(text); setText(""); }}>
          <input placeholder="Tambah tugas, tekan Enter…" value={text} maxLength={120} onChange={(e) => setText(e.target.value)} {...typing} />
        </form>
        <div className="todos">
          {todos.todos.length === 0 && <div className="sempty">Belum ada tugas.</div>}
          {todos.todos.map((t) => (
            <div key={t.id} className="todo" data-done={t.done ? "1" : undefined}>
              <button type="button" className="chk" aria-label="Selesai" onClick={() => todos.toggle(t.id)}>{t.done && <Icon name="check" size={12} sw={3} />}</button>
              <span>{t.text}</span>
              <IconButton name="x" size={12} label="Hapus" className="xs" onClick={() => todos.remove(t.id)} />
            </div>
          ))}
        </div>
      </Sec>
      <Sec title="Catatan cepat">
        <textarea className="note" placeholder="Tulis apa saja… tersimpan otomatis di perangkat ini." value={todos.note} onChange={(e) => todos.setNote(e.target.value)} {...typing} />
      </Sec>
    </>
  );
}

// ───────────────────────── Alat ─────────────────────────
function useAction() {
  const [msg, setMsg] = useState({});
  const run = async (key, fn, fmt) => {
    setMsg((p) => ({ ...p, [key]: "…" }));
    const r = await fn();
    setMsg((p) => ({ ...p, [key]: fmt(r) }));
    setTimeout(() => setMsg((p) => ({ ...p, [key]: null })), 5000);
  };
  return [msg, run];
}

export function Tools({ d }) {
  const [procs, setProcs] = useState([]);
  const [armed, setArmed] = useState(null);
  const [msg, run] = useAction();
  const load = () => call("top_processes", { n: 8 }).then((r) => Array.isArray(r) && setProcs(r));
  useEffect(() => { load(); }, []);
  const kill = async (p) => {
    if (armed !== p.name) {
      setArmed(p.name);
      setTimeout(() => setArmed((a) => (a === p.name ? null : a)), 2500);
      return;
    }
    setArmed(null);
    await call("kill_process", { name: p.name });
    setTimeout(load, 500);
  };
  const tools = [
    ["Device Manager", "devmgmt.msc", "cpu"],
    ["Layanan", "services.msc", "gear"],
    ["Resource Monitor", "resmon", "activity"],
    ["Jaringan", "ncpa.cpl", "wifi"],
    ["Opsi daya", "powercfg.cpl", "zap"],
    ["Penyimpanan", "ms-settings:storagesense", "shelf"],
  ];
  return (
    <>
      <Sec title="Proses terberat (RAM)" action={<button type="button" className="mini-act" onClick={load}>Segarkan</button>}>
        {procs.map((p) => (
          <div key={p.name} className="prow">
            <b title={p.name}>{p.name.replace(/\.exe$/i, "")}{p.count > 1 ? <i> ×{p.count}</i> : null}</b>
            <span className="num">{p.memMb >= 1024 ? `${(p.memMb / 1024).toFixed(1)} GB` : `${p.memMb} MB`}</span>
            <button type="button" className="kill" data-armed={armed === p.name ? "1" : undefined} disabled={p.protected} title={p.protected ? "Proses sistem dilindungi" : "Akhiri proses"} onClick={() => kill(p)}>
              {p.protected ? "🔒" : armed === p.name ? "Yakin?" : "Akhiri"}
            </button>
          </div>
        ))}
        {procs.length === 0 && <div className="sempty">Memuat…</div>}
      </Sec>
      <Sec title="Pembersih & perbaikan">
        <div className="toolgrid">
          <button type="button" onClick={() => run("ram", () => call("clean_ram"), (r) => (r ? `${r.freedMb} MB dibebaskan` : "gagal"))}>
            <Icon name="broom" size={16} /> <b>Bersihkan RAM</b><span>{msg.ram || "Kosongkan working set"}</span>
          </button>
          <button type="button" onClick={() => run("tmp", () => call("clean_temp"), (r) => (r == null ? "gagal" : `${(r / 1048576).toFixed(r > 1e8 ? 0 : 1)} MB dibersihkan`))}>
            <Icon name="trash" size={16} /> <b>Bersihkan Temp</b><span>{msg.tmp || "File > 3 hari"}</span>
          </button>
          <button type="button" onClick={() => run("dns", () => call("flush_dns"), (r) => (r ? "Cache DNS dikosongkan" : "gagal"))}>
            <Icon name="wifi" size={16} /> <b>Flush DNS</b><span>{msg.dns || "Perbaiki koneksi lambat"}</span>
          </button>
          <button type="button" onClick={() => confirm("Restart Windows Explorer? Taskbar akan berkedip sesaat.") && run("exp", () => call("restart_explorer"), (r) => (r ? "Explorer dimulai ulang" : "gagal"))}>
            <Icon name="reset" size={16} /> <b>Restart Explorer</b><span>{msg.exp || "Atasi taskbar macet"}</span>
          </button>
        </div>
      </Sec>
      <Sec title="Alat sistem">
        <div className="links">
          {tools.map(([n, c, ic]) => (
            <button key={c} type="button" onClick={() => call("launch", { path: c })}><Icon name={ic} size={14} /> {n}</button>
          ))}
        </div>
      </Sec>
    </>
  );
}
