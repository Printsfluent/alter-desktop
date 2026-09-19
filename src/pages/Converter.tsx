// Bd — the single shared converter component. kind='image'|'video'. Entirely
// separate from the spoof pipeline: raw paths are passed straight to the
// convert_* backend commands (stage=false, no stage_file_upload). quality is
// always null on the image path (no quality slider exists in this UI).
import { useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { T, isTauri } from "../lib/tauri";
import { Gv, Kv, Jv, Yv, PAGE_SIZE } from "../lib/constants";
import { qi, jl, extname, basename, Wg, pad3 } from "../lib/helpers";
import { DropZone } from "../components/DropZone";

interface Staged { path: string; name: string }

// Parent folder of an absolute output path (for "Open folder").
function folderOf(p: string): string {
  const b = basename(p);
  const cut = p.slice(0, p.length - b.length).replace(/[\\/]+$/, "");
  return cut || p;
}

export function Converter({ kind }: { kind: "image" | "video" }) {
  const isImage = kind === "image";
  const inputExts = isImage ? Gv : Jv;
  const targets = isImage ? Kv : Yv;

  const [files, setFiles] = useState<Staged[]>([]);
  const [format, setFormat] = useState(isImage ? "jpg" : "mp4");
  const [dir, setDir] = useState<string | null>(null); // chosen export dir
  const [page, setPage] = useState(0);
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<string[]>([]); // produced output paths
  const [toast, setToast] = useState<string | null>(null);

  const flash = (m: string) => { setToast(m); window.setTimeout(() => setToast(null), 3000); };

  // Reset per-kind derived state when switching image <-> video.
  useEffect(() => {
    setFormat(isImage ? "jpg" : "mp4");
    setFiles([]);
    setPage(0);
    setResults([]);
  }, [kind]);

  // ---- staging (raw paths accumulate) ----
  const onStaged = (paths: string[], names: string[]) => {
    setFiles((prev) => [...prev, ...paths.map((p, i) => ({ path: p, name: names[i] }))]);
  };

  const removeAt = (k: number) => {
    const next = files.filter((_, i) => i !== k);
    setFiles(next);
    const maxPage = Math.max(0, Math.ceil(next.length / PAGE_SIZE) - 1);
    setPage((p) => Math.min(p, maxPage));
  };

  const clearAll = () => { setFiles([]); setPage(0); };

  // ---- export dir ----
  const chooseDir = async () => {
    if (!isTauri()) { flash("Open the desktop app to choose a folder."); return; }
    try {
      const d = await openDialog({ directory: true, multiple: false });
      if (typeof d === "string") setDir(d);
    } catch (e) {
      flash(String(e));
    }
  };

  // ---- command wrappers (quality is ALWAYS null for image) ----
  const convertImage = (inputPath: string, baseName: string, outputDir: string | null) =>
    T<string>("convert_image_file", { inputPath, targetFormat: format, quality: null, baseName, outputDir });
  const convertVideo = (inputPath: string, baseName: string, outputDir: string | null) =>
    T<string>("convert_video_file", { inputPath, targetFormat: format, baseName, outputDir });

  // ---- run ----
  const convert = async () => {
    if (files.length === 0 || busy) return;
    setBusy(true);
    const outputDir = dir ? Wg(dir, "Converted") : null;
    const produced: string[] = [];
    try {
      for (let i = 0; i < files.length; i++) {
        const f = files[i];
        const stem = qi(jl(f.name));
        const baseName = files.length > 1 ? `${stem}_${pad3(i + 1)}` : stem;
        try {
          const out = isImage
            ? await convertImage(f.path, baseName, outputDir)
            : await convertVideo(f.path, baseName, outputDir);
          if (out) produced.push(out);
        } catch (e) {
          flash(String(e));
        }
      }
      if (produced.length) setResults((prev) => [...produced, ...prev]);
    } finally {
      setBusy(false);
    }
  };

  const pageCount = Math.max(1, Math.ceil(files.length / PAGE_SIZE));
  const pageFiles = files.slice(page * PAGE_SIZE, page * PAGE_SIZE + PAGE_SIZE);

  return (
    <div className="page">
      <div className="page-header">
        <div className="eyebrow">{isImage ? "Convert" : "Video editing"}</div>
        <h1>
          {isImage ? "Image converter" : "Video converter"} <span className="badge">Batch</span>
        </h1>
        <p className="control-hint">
          {isImage
            ? "Convert images between formats in bulk — pick a target and drop your files."
            : "Transcode videos between formats in bulk — pick a target and drop your files."}
        </p>
      </div>

      {/* drop zone */}
      <div className="card">
        <DropZone
          onStaged={onStaged}
          extensions={inputExts}
          label="Drop files to convert"
          multiple
          stage={false}
          disabled={busy}
          imageOnly={isImage}
          videoOnly={!isImage}
        />
      </div>

      {/* staged file list */}
      {files.length > 0 && (
        <div className="card">
          <div className="card-head">
            <div className="card-title">{files.length} file(s) staged</div>
            <button className="btn ghost small" onClick={clearAll} disabled={busy}>Clear</button>
          </div>
          {pageFiles.map((f, i) => {
            const idx = page * PAGE_SIZE + i;
            return (
              <div className="list-row" key={idx}>
                <div className="row-main">
                  <div className="row-name">{f.name}</div>
                  <div className="row-sub">{extname(f.name).toUpperCase()} → {format.toUpperCase()}</div>
                </div>
                <button className="icon-btn" onClick={() => removeAt(idx)} disabled={busy}>✕</button>
              </div>
            );
          })}
          {files.length > PAGE_SIZE && (
            <div className="pager">
              <button disabled={page <= 0} onClick={() => setPage((p) => p - 1)}>← Prev</button>
              <span>Page {page + 1} / {pageCount}</span>
              <button disabled={(page + 1) * PAGE_SIZE >= files.length} onClick={() => setPage((p) => p + 1)}>Next →</button>
            </div>
          )}
        </div>
      )}

      {/* options */}
      <div className="card">
        <div className="card-title">Options</div>

        <div className="control-row">
          <div className="control-label"><span className="lbl">Output format</span></div>
          <div className="format-toggle">
            {targets.map((t) => (
              <button
                key={t}
                type="button"
                className={`btn small${t === format ? " active" : ""}`}
                disabled={busy}
                onClick={() => setFormat(t)}
              >
                {t.toUpperCase()}
              </button>
            ))}
          </div>
        </div>

        <div className="control-row">
          <div className="control-label">
            <span className="lbl">Export folder</span>
            <span className="control-hint">{dir ? `${basename(dir)}/Converted` : "Default folder"}</span>
          </div>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button className="btn ghost small" onClick={chooseDir} disabled={busy}>Change…</button>
            {dir && <button className="btn ghost small" onClick={() => setDir(null)} disabled={busy}>Reset</button>}
          </div>
        </div>
      </div>

      {/* convert */}
      {files.length > 0 && (
        <div className="card" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div className="control-hint">Converting to {format.toUpperCase()} · {files.length} file(s)</div>
          <button className="btn primary" onClick={convert} disabled={busy || files.length === 0}>
            {busy ? "Converting..." : `Convert ${files.length} file(s)`}
          </button>
        </div>
      )}

      {/* results */}
      {results.length > 0 && (
        <div className="card">
          <div className="card-head">
            <div className="card-title">Converted ({results.length})</div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button className="btn ghost small" onClick={() => openPath(folderOf(results[0]))}>Open folder</button>
              <button className="btn ghost small" onClick={() => setResults([])}>Clear</button>
            </div>
          </div>
          {results.map((p, i) => (
            <div className="list-row" key={i}>
              <div className="row-main">
                <div className="row-name">{basename(p)}</div>
                <div className="row-sub">{p}</div>
              </div>
              <button className="icon-btn" onClick={() => openPath(p)}>Open</button>
            </div>
          ))}
        </div>
      )}

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
