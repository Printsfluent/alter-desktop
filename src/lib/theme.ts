// Theme = a `.dark` class on <html>, seeded from localStorage['alter-theme'].
import { Nf, ag } from "./constants";

export type Theme = "light" | "dark";

function storedTheme(): string | null {
  try { return localStorage.getItem(Nf); }
  catch { return null; }
}

function persistTheme(t: Theme) {
  try { localStorage.setItem(Nf, t); }
  catch { /* private / blocked storage */ }
}

// ug() — initial theme
export function readTheme(): Theme {
  const v = storedTheme();
  return v === "light" || v === "dark" ? v : (ag as Theme);
}

// cg(l) — apply + persist
export function applyTheme(t: Theme) {
  document.documentElement.classList.toggle("dark", t === "dark");
  persistTheme(t);
}
