// Qg — the /settings page. Output folder, performance, and about cards.
import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { T, isTauri } from "../lib/tauri";
import { xf } from "../lib/constants";

// last path segment (handles both slash styles) for a friendly folder label.
function baseName(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || p;
}

export function Settings() {
  const [outputDir, setOutputDir] = useState<string | null>(null);
  const [concurrency, setConcurrency] = useState(2);
  const [toast, setToast] = useState<string | null>(null);

  const flash = (m: string) => { setToast(m); window.setTimeout(() => setToast(null), 3000); };

  useEffect(() => {
    T<string | null>("get_output_dir").then((d) => setOutputDir(d)).catch(() => {});
    T<number>("get_concurrency").then((v) => setConcurrency(v)).catch(() => {});
  }, []);

  // ---- output folder ----
  const changeDir = async () => {
    if (!isTauri()) { flash("Open the desktop app to choose a folder."); return; }
    try {
      const dir = await openDialog({ directory: true, multiple: false });
      if (typeof dir === "string") {
        await T("set_output_dir", { dir });
        setOutputDir(dir);
        flash("Output folder updated.");
      }
    } catch (e) { flash(String(e)); }
  };

  const resetDir = async () => {
    try {
      await T("set_output_dir", { dir: null });
      setOutputDir(null);
      flash("Output folder reset to default.");
    } catch (e) { flash(String(e)); }
  };

  const openOutputs = () =>
    T("open_outputs_folder", { platform: "instagram" }).catch((e) => flash(String(e)));

  // ---- concurrency ----
  const onConcurrency = (raw: number) => {
    if (isNaN(raw)) return;
    const value = Math.max(1, Math.min(8, Math.round(raw)));
    setConcurrency(value);
    T("set_concurrency", { value }).catch((e) => flash(String(e)));
  };

  return (
    <div className="page">
      <div className="page-header">
        <div className="eyebrow">Settings</div>
        <h1>Settings</h1>
      </div>

      {/* ---- Output ---- */}
      <div className="card">
        <div className="card-title">Output</div>
        <div className="control-row">
          <div className="control-label">
            <span className="lbl">Output folder</span>
            <span className="control-hint">{outputDir ? baseName(outputDir) : "App default"}</span>
          </div>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button className="btn ghost small" onClick={changeDir}>Change…</button>
            {outputDir && <button className="btn ghost small" onClick={resetDir}>Reset to default</button>}
          </div>
        </div>
        <div className="control-row">
          <div className="control-label">
            <span className="lbl">Open folder</span>
            <span className="control-hint">Reveal your spoofed outputs in the file manager</span>
          </div>
          <button className="btn ghost small" onClick={openOutputs}>Open</button>
        </div>
      </div>

      {/* ---- Performance ---- */}
      <div className="card">
        <div className="card-title">Performance</div>
        <div className="control-row">
          <div className="control-label">
            <span className="lbl">Parallel spoofs</span>
            <span className="control-hint">How many outputs render at once (1–8)</span>
          </div>
          <input
            className="text-input"
            type="number"
            min={1}
            max={8}
            step={1}
            value={concurrency}
            style={{ width: 80, textAlign: "center" }}
            onChange={(e) => onConcurrency(parseInt(e.target.value, 10))}
          />
        </div>
      </div>

      {/* ---- About ---- */}
      <div className="card">
        <div className="card-title">About</div>

        <div className="control-row">
          <div className="control-label"><span className="lbl">App</span></div>
          <span className="muted">{xf}</span>
        </div>
        <div className="control-row">
          <div className="control-label"><span className="lbl">Version</span></div>
          <span className="muted">2.0.1</span>
        </div>
        <div className="control-row">
          <div className="control-label"><span className="lbl">Identifier</span></div>
          <span className="muted">com.alter.desktop</span>
        </div>
        <div className="control-row">
          <div className="control-label"><span className="lbl">Description</span></div>
          <span className="muted">Media creator desktop suite</span>
        </div>
      </div>

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
