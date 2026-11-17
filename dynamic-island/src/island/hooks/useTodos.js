import { useCallback, useState } from "react";

const KEY = "di.todos.v1";
const NOTE = "di.note.v1";
const load = (k, d) => {
  try {
    return JSON.parse(localStorage.getItem(k) ?? "null") ?? d;
  } catch {
    return d;
  }
};
const save = (k, v) => {
  try {
    localStorage.setItem(k, JSON.stringify(v));
  } catch {}
};

/** To-do & catatan cepat (localStorage, dibagi antar jendela). */
export function useTodos() {
  const [todos, setTodos] = useState(() => load(KEY, []));
  const [note, setNoteS] = useState(() => load(NOTE, ""));
  const commit = useCallback((fn) => setTodos((p) => { const n = fn(p); save(KEY, n); return n; }), []);
  const add = useCallback((text) => text.trim() && commit((p) => [{ id: Date.now().toString(36), text: text.trim(), done: false }, ...p]), [commit]);
  const toggle = useCallback((id) => commit((p) => p.map((t) => (t.id === id ? { ...t, done: !t.done } : t))), [commit]);
  const remove = useCallback((id) => commit((p) => p.filter((t) => t.id !== id)), [commit]);
  const clearDone = useCallback(() => commit((p) => p.filter((t) => !t.done)), [commit]);
  const setNote = useCallback((v) => { setNoteS(v); save(NOTE, v); }, []);
  return { todos, add, toggle, remove, clearDone, note, setNote };
}
