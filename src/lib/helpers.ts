// Pure helpers mirroring the spec's minified utilities.
import { Ll } from "./constants";

// qi / $d — sanitize: strip disallowed chars, trim, strip leading/trailing dots, fallback "output".
export function qi(s: string | null | undefined): string {
  if (!s) return "output";
  let out = s.replace(/[^\p{L}\p{N} \-_.]/gu, "_").trim().replace(/^\.+|\.+$/g, "");
  return out || "output";
}

// jl — strip extension.
export function jl(name: string): string {
  const i = name.lastIndexOf(".");
  return i > 0 ? name.slice(0, i) : name;
}

// extension (lowercased, no dot)
export function extname(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}

export function basename(p: string): string {
  const parts = p.split(/[\\/]/);
  return parts[parts.length - 1] || p;
}

// Wg / Fg — path join with a subfolder.
export function Wg(dir: string, sub: string): string {
  const sep = dir.includes("\\") ? "\\" : "/";
  const trimmed = dir.replace(/[\\/]+$/, "");
  return `${trimmed}${sep}${sub}`;
}

// aa — classify a file by extension: image if in Ll, else video.
export function aa(name: string): "image" | "video" {
  return Ll.includes(extname(name)) ? "image" : "video";
}

// ir — mm:ss time format.
export function ir(s: number): string {
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}

// zero-pad to 3
export function pad3(n: number): string {
  return String(n).padStart(3, "0");
}
