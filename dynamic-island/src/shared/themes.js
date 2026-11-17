import { useLayoutEffect } from "react";
import { hexToRgb } from "./color";
import { useSettings } from "./settings";

/** Preset tema. bg = RGB latar kapsul/dock, accent = RGB aksen bawaan, mode = dark|light. */
export const THEMES = {
  obsidian: { name: "Obsidian", mode: "dark", bg: "12,12,14", panel: "#0c0c0f", panel2: "#09090c", accent: "245,245,247", sw: ["#17171b", "#f5f5f7"] },
  carbon: { name: "Carbon", mode: "dark", bg: "0,0,0", panel: "#000000", panel2: "#000000", accent: "226,226,232", sw: ["#000000", "#9a9aa6"] },
  aurora: { name: "Aurora", mode: "dark", bg: "8,14,26", panel: "#070c16", panel2: "#050912", accent: "125,211,252", sw: ["#0b1830", "#7dd3fc"] },
  royal: { name: "Royal", mode: "dark", bg: "14,11,28", panel: "#0d0a1b", panel2: "#09071a", accent: "196,181,253", sw: ["#1a1238", "#c4b5fd"] },
  mint: { name: "Mint", mode: "dark", bg: "8,18,16", panel: "#08120f", panel2: "#060e0c", accent: "110,231,183", sw: ["#0b231e", "#6ee7b7"] },
  sunset: { name: "Sunset", mode: "dark", bg: "24,13,10", panel: "#160c0a", panel2: "#100807", accent: "253,186,116", sw: ["#2a140c", "#fdba74"] },
  sakura: { name: "Sakura", mode: "dark", bg: "24,12,19", panel: "#160a10", panel2: "#10070b", accent: "249,168,212", sw: ["#2a1020", "#f9a8d4"] },
  porcelain: { name: "Porcelain", mode: "light", bg: "250,250,253", panel: "#f3f3f8", panel2: "#e9e9f1", accent: "37,99,235", sw: ["#fafafd", "#2563eb"] },
};

export const ACCENT_PRESETS = ["auto", "theme", "#f5f5f7", "#7dd3fc", "#6ee7b7", "#fdba74", "#c4b5fd", "#fda4af", "#facc15"];

/** Terapkan token tema ke :root. Dipanggil di setiap jendela (island, dock, sidebar, pengaturan). */
export function applyTheme(s) {
  const t = s.theme;
  const th = THEMES[t.id] || THEMES.obsidian;
  const light = th.mode === "light";
  const manual = /^#/.test(t.accent) ? hexToRgb(t.accent) : null;
  const root = document.documentElement;
  const v = {
    "--bg-rgb": th.bg,
    "--bg-a": String(t.glass),
    "--dock-a": String(Math.max(0.55, t.glass - 0.04)),
    "--fg-rgb": light ? "0,0,0" : "255,255,255",
    "--text": light ? "#0b0b0d" : "#f5f5f7",
    "--text-2": light ? "rgba(11,11,13,.66)" : "rgba(245,245,247,.64)",
    "--text-3": light ? "rgba(11,11,13,.44)" : "rgba(245,245,247,.4)",
    "--on-accent": light ? "#ffffff" : "#0b0b0d",
    "--panel": th.panel,
    "--panel-2": th.panel2,
    "--glow": String(t.glow),
    "--accent-rgb": manual ? manual.join(",") : th.accent,
  };
  for (const k of Object.keys(v)) root.style.setProperty(k, v[k]);
  root.dataset.theme = t.id;
  root.dataset.mode = th.mode;
  root.style.colorScheme = th.mode;
}

export function useTheme() {
  const s = useSettings();
  useLayoutEffect(() => applyTheme(s), [s.theme]); // eslint-disable-line
}
