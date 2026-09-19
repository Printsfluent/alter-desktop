// Thin IPC layer. Mirrors the spec's invoke wrapper `T(name,args)` and event bus.
import { invoke as tauriInvoke, convertFileSrc as tauriConvert } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// T(name, args) => __TAURI_INTERNALS__.invoke(name, args)
export function T<R = unknown>(name: string, args?: Record<string, unknown>): Promise<R> {
  if (!isTauri()) return Promise.reject(new Error("Desktop backend is not available in the browser."));
  return tauriInvoke<R>(name, args);
}

export function listen<P = unknown>(event: string, cb: (payload: P) => void): Promise<UnlistenFn> {
  if (!isTauri()) return Promise.resolve(() => {});
  return tauriListen<P>(event, (e) => cb(e.payload as P));
}

// convertFileSrc(path,'asset') → asset: URL for previewing local media.
export function convertFileSrc(path: string, protocol = "asset"): string {
  if (!isTauri()) return path;
  try {
    return tauriConvert(path, protocol);
  } catch {
    return path;
  }
}
