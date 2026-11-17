// Definisi hotkey global. mods Win32: ALT=1, CTRL=2, SHIFT=4, WIN=8.
export const ACTIONS = [
  { id: "toggle", label: "Tampil / sembunyikan semua", def: "Ctrl+Alt+I", group: "Umum" },
  { id: "settings", label: "Buka pengaturan", def: "Ctrl+Alt+,", group: "Umum" },
  { id: "sidebar", label: "Tampil / sembunyikan sidebar", def: "Ctrl+Alt+S", group: "Umum" },
  { id: "dock", label: "Tampil / sembunyikan dock", def: "Ctrl+Alt+D", group: "Umum" },
  { id: "clipboard", label: "Buka riwayat clipboard", def: "Ctrl+Alt+V", group: "Panel" },
  { id: "game", label: "Buka panel Game", def: "Ctrl+Alt+G", group: "Panel" },
  { id: "boost", label: "Game Boost aktif / nonaktif", def: "Ctrl+Alt+B", group: "Game" },
  { id: "screenshot", label: "Tangkap layar", def: "Ctrl+Alt+P", group: "Utilitas" },
  { id: "lock", label: "Kunci komputer", def: "Ctrl+Alt+L", group: "Utilitas" },
  { id: "media_play", label: "Putar / jeda", def: "Ctrl+Alt+Space", group: "Media" },
  { id: "media_next", label: "Lagu berikutnya", def: "Ctrl+Alt+Right", group: "Media" },
  { id: "media_prev", label: "Lagu sebelumnya", def: "Ctrl+Alt+Left", group: "Media" },
  { id: "vol_up", label: "Volume naik", def: "Ctrl+Alt+Up", group: "Media" },
  { id: "vol_down", label: "Volume turun", def: "Ctrl+Alt+Down", group: "Media" },
  { id: "mute", label: "Bisukan", def: "Ctrl+Alt+M", group: "Media" },
];

const NAMED = { Space: 0x20, Up: 0x26, Down: 0x28, Left: 0x25, Right: 0x27, Enter: 0x0d, Tab: 0x09, Home: 0x24, End: 0x23, PageUp: 0x21, PageDown: 0x22, Insert: 0x2d, Delete: 0x2e, ",": 0xbc, ".": 0xbe, "/": 0xbf, ";": 0xba, "'": 0xde, "[": 0xdb, "]": 0xdd, "\\": 0xdc, "-": 0xbd, "=": 0xbb, "`": 0xc0 };
const CODE_TO_TOKEN = { Space: "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right", Enter: "Enter", Tab: "Tab", Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown", Insert: "Insert", Delete: "Delete", Comma: ",", Period: ".", Slash: "/", Semicolon: ";", Quote: "'", BracketLeft: "[", BracketRight: "]", Backslash: "\\", Minus: "-", Equal: "=", Backquote: "`" };

const keyVk = (t) => {
  if (/^[A-Z]$/.test(t)) return t.charCodeAt(0);
  if (/^[0-9]$/.test(t)) return t.charCodeAt(0);
  if (/^F([1-9]|1[0-2])$/.test(t)) return 0x6f + Number(t.slice(1));
  return NAMED[t] ?? null;
};

export function parseCombo(str) {
  if (!str) return null;
  const parts = str.split("+");
  // token terakhir adalah tombol (bisa "+" sendiri tidak didukung)
  const key = str.endsWith("+,") ? "," : parts[parts.length - 1];
  const mods = (str.includes("Alt") ? 1 : 0) | (str.includes("Ctrl") ? 2 : 0) | (str.includes("Shift") ? 4 : 0) | (str.includes("Win") ? 8 : 0);
  const vk = keyVk(key);
  return vk && mods ? { mods, vk } : null;
}

/** Bangun string kombinasi dari KeyboardEvent (wajib ada minimal satu modifier). */
export function comboFromEvent(e) {
  let key = null;
  if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (/^Digit[0-9]$/.test(e.code)) key = e.code.slice(5);
  else if (/^F([1-9]|1[0-2])$/.test(e.code)) key = e.code;
  else key = CODE_TO_TOKEN[e.code] ?? null;
  if (!key) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
  return mods.length ? [...mods, key].join("+") : null;
}

export const comboOf = (map, a) => (a.id in map ? map[a.id] : a.def);

/** Daftar item untuk backend: aksi yang bisa diatur + Ctrl+Alt+1…9 (luncurkan item dock). */
export function buildItems(map) {
  const out = [];
  for (const a of ACTIONS) {
    const c = comboOf(map, a);
    const p = parseCombo(c);
    if (p) out.push({ name: a.id, mods: p.mods, vk: p.vk });
  }
  for (let i = 1; i <= 9; i++) out.push({ name: `dock-${i}`, mods: 3, vk: 0x30 + i });
  return out;
}
