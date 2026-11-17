import { useEffect, useState } from "react";
import { Icon, IconButton } from "../shared/Icon";
import { call } from "../shared/tauri";
import { updateSettings } from "../shared/settings";
import { ACCENT_PRESETS, THEMES } from "../shared/themes";
import { ACTIONS, comboFromEvent, comboOf } from "../shared/hotkeys";
import { AppList, Group, Range, Row, Seg, Toggle, pickApp } from "./controls";
import { stem } from "../shared/format";

// ───────────────────────── Tema ─────────────────────────
export function ThemePage({ s }) {
  const t = s.theme;
  const set = (p) => updateSettings({ theme: p });
  const th = THEMES[t.id] || THEMES.obsidian;
  return (
    <>
      <div className="tprev-wrap">
        <div className="tprev" style={{ "--bg-rgb": th.bg, "--fg-rgb": th.mode === "light" ? "0,0,0" : "255,255,255", "--bg-a": t.glass, "--glow": t.glow, "--accent-rgb": /^#/.test(t.accent) ? hexRgb(t.accent) : th.accent, color: th.mode === "light" ? "#0b0b0d" : "#f5f5f7" }}>
          <span className="tp-dot" />
          <b>Midnight City</b>
          <i>M83</i>
          <span className="tp-bars"><u /><u /><u /><u /></span>
        </div>
        <div className="tprev tdock" style={{ "--bg-rgb": th.bg, "--fg-rgb": th.mode === "light" ? "0,0,0" : "255,255,255", "--bg-a": t.glass, "--glow": t.glow, "--accent-rgb": /^#/.test(t.accent) ? hexRgb(t.accent) : th.accent }}>
          {["#ef4444", "#3b82f6", "#22c55e", "#f59e0b", "#a855f7"].map((c) => <em key={c} style={{ background: c }} />)}
        </div>
      </div>
      <Group title="Tema">
        <div className="themes">
          {Object.entries(THEMES).map(([id, x]) => (
            <button key={id} type="button" className="tcard" data-on={t.id === id ? "1" : undefined} onClick={() => set({ id })}>
              <span className="tsw" style={{ background: `linear-gradient(135deg, ${x.sw[0]} 55%, ${x.sw[1]})` }} />
              <b>{x.name}</b>
              <em>{x.mode === "light" ? "Terang" : "Gelap"}</em>
            </button>
          ))}
        </div>
      </Group>
      <Group title="Warna aksen">
        <Row title="Aksen" desc="“Otomatis” mengambil warna dominan sampul album di island; “Tema” memakai aksen bawaan tema.">
          <div className="sw">
            {ACCENT_PRESETS.map((c) => (
              <button key={c} type="button" className={`swatch ${c === "auto" ? "auto" : c === "theme" ? "themed" : ""}`} data-on={t.accent === c ? "1" : undefined} style={/^#/.test(c) ? { background: c } : c === "theme" ? { background: `rgb(${th.accent})` } : undefined} title={c === "auto" ? "Otomatis dari sampul" : c === "theme" ? "Bawaan tema" : c} onClick={() => set({ accent: c })}>
                {c === "auto" && <Icon name="sparkle" size={13} />}
              </button>
            ))}
            <label className="swatch pick" title="Warna kustom">
              <input type="color" value={/^#/.test(t.accent) ? t.accent : "#7dd3fc"} onChange={(e) => set({ accent: e.target.value })} />
              <Icon name="plus" size={13} />
            </label>
          </div>
        </Row>
      </Group>
      <Group title="Permukaan & gerak">
        <Row title="Kepekatan latar" desc="Kiri = kaca tembus pandang, kanan = solid (lebih hemat GPU)."><Range value={t.glass} min={0.6} max={1} step={0.02} fmt={(v) => `${Math.round(v * 100)}%`} onChange={(v) => set({ glass: v })} /></Row>
        <Row title="Cahaya tepi (glow)" desc="Intensitas garis gradien di tepi kapsul, dock, dan sidebar."><Range value={t.glow} min={0} max={1} step={0.05} fmt={(v) => `${Math.round(v * 100)}%`} onChange={(v) => set({ glow: v })} /></Row>
        <Row title="Kecepatan animasi" desc="Mengatur kekakuan spring saat kapsul berubah bentuk."><Range value={t.anim} min={0.6} max={1.6} step={0.1} fmt={(v) => `${v.toFixed(1)}×`} onChange={(v) => set({ anim: v })} /></Row>
      </Group>
    </>
  );
}
const hexRgb = (h) => {
  const n = parseInt(h.slice(1), 16);
  return `${(n >> 16) & 255},${(n >> 8) & 255},${n & 255}`;
};

// ───────────────────────── Game ─────────────────────────
export function GamePage({ s }) {
  const g = s.game;
  const set = (p) => updateSettings({ game: p });
  const [procs, setProcs] = useState(null);
  const [name, setName] = useState("");
  const addLib = async () => {
    const p = await pickApp();
    if (p && !g.library.some((x) => x.path === p)) set({ library: [...g.library, { path: p, name: stem(p) }] });
  };
  const addClose = (n) => {
    const v = n.trim().toLowerCase();
    if (v && !g.close.includes(v)) set({ close: [...g.close, v.endsWith(".exe") ? v : v + ".exe"] });
    setName("");
  };
  const loadProcs = async () => setProcs((await call("top_processes", { n: 20 }))?.filter((p) => !p.protected) ?? []);
  return (
    <>
      <Group title="Game Boost">
        <Row title="Boost otomatis" desc="Diterapkan saat game fullscreen terdeteksi dan dipulihkan 8 detik setelah selesai. Tidak ada injeksi ke game (aman untuk anti-cheat)."><Toggle on={g.auto} onChange={(v) => set({ auto: v })} label="Boost otomatis" /></Row>
        <Row title="Cakupan deteksi" desc="“Semua fullscreen” mengabaikan browser & pemutar video."><Seg value={g.scope} options={[["all", "Semua fullscreen"], ["list", "Hanya library"]]} onChange={(v) => set({ scope: v })} /></Row>
        <Row title="Power plan Performa maksimum" desc="Beralih ke High performance lalu mengembalikan skema semula."><Toggle on={g.power} onChange={(v) => set({ power: v })} label="Power plan" /></Row>
        <Row title="Mode senyap" desc="Mematikan banner notifikasi Windows selama bermain."><Toggle on={g.dnd} onChange={(v) => set({ dnd: v })} label="Senyap" /></Row>
        <Row title="Windows Game Mode"><Toggle on={g.gameMode} onChange={(v) => set({ gameMode: v })} label="Game Mode" /></Row>
        <Row title="Prioritas proses tinggi" desc="Menaikkan prioritas CPU proses game."><Toggle on={g.priority} onChange={(v) => set({ priority: v })} label="Prioritas" /></Row>
        <Row title="Bersihkan RAM saat boost" desc="Kosongkan working set proses lain. Efek sementara; berguna jika RAM hampir penuh."><Toggle on={g.cleanRam} onChange={(v) => set({ cleanRam: v })} label="Bersihkan RAM" /></Row>
      </Group>
      <Group title="Aplikasi yang diakhiri saat boost">
        <div className="note">Pilihan Anda sendiri — aplikasi ini TIDAK menutup apa pun kecuali yang tercantum di sini. Simpan pekerjaan dulu; proses sistem dilindungi.</div>
        <div className="chipbox">
          {g.close.map((c) => (
            <span key={c} className="xchip">{c}<button type="button" aria-label="Hapus" onClick={() => set({ close: g.close.filter((x) => x !== c) })}><Icon name="x" size={11} sw={3} /></button></span>
          ))}
          {g.close.length === 0 && <span className="hintx">Belum ada.</span>}
        </div>
        <div className="toolbar">
          <input className="txt grow" placeholder="nama exe, mis. OneDrive.exe" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && addClose(name)} />
          <button type="button" className="btn" onClick={() => addClose(name)}><Icon name="plus" size={14} /> Tambah</button>
          <button type="button" className="btn ghosty" onClick={loadProcs}>Pilih dari proses berjalan</button>
        </div>
        {procs && (
          <div className="chipbox pick">
            {procs.map((p) => (
              <button key={p.name} type="button" className="pchip" onClick={() => addClose(p.name)}>{p.name.replace(/\.exe$/i, "")} <em>{p.memMb} MB</em></button>
            ))}
          </div>
        )}
      </Group>
      <Group title="Library game">
        <div className="toolbar">
          <button type="button" className="btn" onClick={addLib}><Icon name="plus" size={14} /> Tambah game…</button>
        </div>
        <AppList items={g.library} onChange={(library) => set({ library })} empty="Tambahkan game (exe atau pintasan). Muncul di sidebar dengan waktu bermain, dan dikenali untuk Boost." />
      </Group>
      <Group title="Jaringan">
        <Row title="Target ping" desc="Host IPv4 untuk pengukuran latensi di panel Game.">
          <div className="rc">
            {["1.1.1.1", "8.8.8.8", "9.9.9.9"].map((h) => (
              <button key={h} type="button" className="chip2" data-on={g.pingHost === h ? "1" : undefined} onClick={() => set({ pingHost: h })}>{h}</button>
            ))}
            <input className="txt" style={{ width: 130 }} value={g.pingHost} onChange={(e) => set({ pingHost: e.target.value.trim() })} />
          </div>
        </Row>
        <div className="note">FPS tidak ditampilkan: mengukur FPS memerlukan hook/ETW yang tidak sejalan dengan prinsip aman-anti-cheat. Gunakan Xbox Game Bar (Win+G) atau PresentMon bila perlu. GPU hanya terbaca untuk kartu NVIDIA.</div>
      </Group>
    </>
  );
}

// ───────────────────────── Sidebar ─────────────────────────
const SEC_NAMES = { home: "Dasbor", game: "Game", apps: "Aplikasi", tasks: "Tugas", tools: "Alat" };
export function SidePage({ s }) {
  const sd = s.side;
  const set = (p) => updateSettings({ side: p });
  const on = sd.sections.filter((k) => SEC_NAMES[k]);
  const all = [...on, ...Object.keys(SEC_NAMES).filter((k) => !on.includes(k))];
  const move = (k, d) => {
    const a = [...on];
    const i = a.indexOf(k);
    const j = i + d;
    if (i < 0 || j < 0 || j >= a.length) return;
    [a[i], a[j]] = [a[j], a[i]];
    set({ sections: a });
  };
  return (
    <>
      <Group title="Sidebar">
        <Row title="Aktifkan sidebar" desc="Panel samping dengan dasbor, kontrol game, peluncur, tugas, dan alat. Dibuat saat dihidupkan dan DIHANCURKAN saat dimatikan (hemat RAM)."><Toggle on={sd.enabled} onChange={(v) => set({ enabled: v })} label="Sidebar" /></Row>
        <Row title="Sisi layar"><Seg value={sd.edge} options={[["left", "Kiri"], ["right", "Kanan"]]} onChange={(v) => set({ edge: v })} /></Row>
        <Row title="Sembunyikan otomatis" desc="Hanya rail ikon yang terlihat; panel terbuka saat hover."><Toggle on={sd.autohide} onChange={(v) => set({ autohide: v })} label="Auto-hide" /></Row>
      </Group>
      <Group title="Bagian">
        {all.map((k) => (
          <Row key={k} title={SEC_NAMES[k]}>
            <IconButton name="chevU" size={14} label="Naik" className="sm" disabled={!on.includes(k) || on[0] === k} onClick={() => move(k, -1)} />
            <IconButton name="chevD" size={14} label="Turun" className="sm" disabled={!on.includes(k) || on.at(-1) === k} onClick={() => move(k, 1)} />
            <Toggle on={on.includes(k)} onChange={(v) => (v ? set({ sections: [...on, k] }) : on.length > 1 && set({ sections: on.filter((x) => x !== k) }))} label={SEC_NAMES[k]} />
          </Row>
        ))}
      </Group>
    </>
  );
}

// ───────────────────────── Hotkeys ─────────────────────────
function KeyCap({ value, def, recording, onRecord, onChange, onCancel, failed }) {
  useEffect(() => {
    if (!recording) return;
    const h = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") return onCancel();
      if (e.key === "Backspace" || e.key === "Delete") return onChange(null);
      const c = comboFromEvent(e);
      if (c) onChange(c);
    };
    window.addEventListener("keydown", h, true);
    return () => window.removeEventListener("keydown", h, true);
  }, [recording]); // eslint-disable-line
  return (
    <button type="button" className="kcap" data-rec={recording ? "1" : undefined} data-bad={failed ? "1" : undefined} onClick={onRecord} title={failed ? "Kombinasi ini sudah dipakai aplikasi lain" : "Klik lalu tekan kombinasi baru (Esc batal, Backspace reset)"}>
      {recording ? "Tekan kombinasi…" : value}
      {value !== def && !recording && <i>diubah</i>}
    </button>
  );
}

export function HotkeysPage({ s }) {
  const h = s.hotkeys;
  const [rec, setRec] = useState(null);
  const [failed, setFailed] = useState(() => { try { return JSON.parse(localStorage.getItem("di.hk.failed") || "[]"); } catch { return []; } });
  const [warn, setWarn] = useState("");
  useEffect(() => {
    const f = (e) => e.key === "di.hk.failed" && setFailed(JSON.parse(e.newValue || "[]"));
    window.addEventListener("storage", f);
    return () => window.removeEventListener("storage", f);
  }, []);
  const set = (id, combo) => {
    const map = { ...h.map };
    if (combo == null) delete map[id];
    else map[id] = combo;
    const dup = ACTIONS.find((a) => a.id !== id && (a.id in map ? map[a.id] : a.def) === (combo ?? ACTIONS.find((x) => x.id === id).def));
    setWarn(dup ? `“${combo ?? "bawaan"}” juga dipakai oleh “${dup.label}”. Salah satunya tidak akan terdaftar.` : "");
    updateSettings({ hotkeys: { map } });
    setRec(null);
  };
  const groups = [...new Set(ACTIONS.map((a) => a.group))];
  return (
    <>
      <Group>
        <Row title="Aktifkan hotkey global" desc="Bekerja di mana saja, termasuk saat game (aksi media/volume/boost ditangani langsung tanpa UI)."><Toggle on={h.enabled} onChange={(v) => updateSettings({ hotkeys: { enabled: v } })} label="Hotkey global" /></Row>
      </Group>
      {warn && <div className="warnbar">{warn}</div>}
      {groups.map((gr) => (
        <Group key={gr} title={gr}>
          {ACTIONS.filter((a) => a.group === gr).map((a) => (
            <Row key={a.id} title={a.label}>
              <KeyCap value={comboOf(h.map, a)} def={a.def} recording={rec === a.id} failed={failed.includes(a.id)} onRecord={() => setRec(rec === a.id ? null : a.id)} onChange={(c) => set(a.id, c)} onCancel={() => setRec(null)} />
              {a.id in h.map && <IconButton name="reset" size={13} label="Kembalikan bawaan" className="sm" onClick={() => set(a.id, null)} />}
            </Row>
          ))}
        </Group>
      ))}
      <Group title="Tetap">
        <Row title="Luncurkan item dock 1–9" desc="Urutan sesuai dock tersemat."><span className="kcap fixed">Ctrl+Alt+1 … 9</span></Row>
      </Group>
    </>
  );
}
