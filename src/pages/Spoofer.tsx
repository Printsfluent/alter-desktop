// ca — the single shared spoofer component. Instagram (u && !c) exposes video/
// image auto-detected modes; Reddit/Twitter (c) use the combined config.
import { useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { T, listen, convertFileSrc, isTauri } from "../lib/tauri";
import {
  Sf, Vv, Hv, rr, Ll, Xv, Qv, maxCopies, Pd, exifButtons, $v, Wv, Bv, PAGE_SIZE,
} from "../lib/constants";
import { qi, jl, aa, Wg, basename } from "../lib/helpers";
import type {
  PlatformId, SpoofConfig, ImageSpoofConfig, RedditSpoofConfig, ExifConfig,
  JobRecord, OutputRecord, ExifDevice, AppPaths,
} from "../lib/types";
import { SegToggle } from "../components/SegToggle";
import { Stepper } from "../components/Stepper";
import { DropZone } from "../components/DropZone";
import { ReviewModal } from "../components/ReviewModal";

interface Batch { paths: string[]; names: string[] }

export function Spoofer({ platformId: l }: { platformId: PlatformId }) {
  const s = Sf.find((p) => p.id === l)!;
  const u = Vv.includes(l);
  const c = Hv.includes(l);

  const [O, setO] = useState<"video" | "image">("video"); // media mode
  const [L, setL] = useState<SpoofConfig>({ ...$v }); // instagram video
  const [W, setW] = useState<ImageSpoofConfig>({ ...Wv }); // instagram image
  const [ee, ie] = useState<RedditSpoofConfig>({ ...Bv }); // reddit/twitter

  const [batch, setBatch] = useState<Batch | null>(null);
  const [outNames, setOutNames] = useState<string[]>([]);
  const [E, setE] = useState(""); // batch folder
  const [B, setB] = useState<string | null>(null); // output dir override
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [defaultOut, setDefaultOut] = useState<string | null>(null); // Te/H
  const [be, setBe] = useState(2); // concurrency
  const [devices, setDevices] = useState<ExifDevice[]>([]);
  const [ae, setAe] = useState(false); // busy
  const [he, setHe] = useState(0); // carousel index
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [outputs, setOutputs] = useState<OutputRecord[]>([]);
  const [outPage, setOutPage] = useState(0);
  const [review, setReview] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const flash = (m: string) => { setToast(m); window.setTimeout(() => setToast(null), 3200); };

  // ---- initial loads + event subscriptions ----
  useEffect(() => {
    T<number>("get_concurrency").then(setBe).catch(() => {});
    T<AppPaths>("get_app_paths").then(setPaths).catch(() => {});
    T<string | null>("get_output_dir").then((d) => setDefaultOut(d)).catch(() => {});
    T<ExifDevice[]>("get_exif_devices").then(setDevices).catch(() => {});
    reloadJobs();
    reloadOutputs();
  }, [l]);

  useEffect(() => {
    const uns: (() => void)[] = [];
    listen<JobRecord>("job-progress", (p) => {
      if ((p as any).platform !== l) return;
      setJobs((prev) => {
        const i = prev.findIndex((j) => j.id === p.id);
        if (i >= 0) { const copy = [...prev]; copy[i] = p; return copy; }
        return [p, ...prev];
      });
    }).then((f) => uns.push(f));
    listen<OutputRecord>("job-completed", (p) => {
      if ((p as any).platform !== l) return;
      reloadOutputs();
      reloadJobs();
    }).then((f) => uns.push(f));
    return () => uns.forEach((f) => f());
  }, [l]);

  const reloadJobs = () => T<JobRecord[]>("list_jobs", { platform: l }).then(setJobs).catch(() => {});
  const reloadOutputs = () => T<OutputRecord[]>("list_outputs", { platform: l }).then(setOutputs).catch(() => {});

  // ---- accepted extensions (Ar) + label (Ur) ----
  const Ar = c ? Xv : u ? (batch ? (O === "image" ? Ll : rr) : [...rr, ...Ll]) : [];
  const Ur = !batch
    ? "Drop a video or image to spoof"
    : O === "image" ? "Drop an image to spoof" : "Drop a video to spoof";

  // Re — outputs per source; gn — the copies/variants label
  const Re = c ? ee.copies : O === "image" ? W.copies : L.outputs_per_source;
  const gn = u && O === "video" && !c ? "Variants" : "Copies";

  // it — EXIF camera visible?  Il — trim visible (reddit: only if a video staged)
  const it = u && (c || O === "image");
  const Il = c ? (batch?.names.some((n) => aa(n) === "video") ?? false) : O === "video";

  const exifCfg: ExifConfig = c ? ee.exif : W.exif;
  const setExif = (patch: Partial<ExifConfig>) => {
    if (c) ie((k) => ({ ...k, exif: { ...k.exif, ...patch } }));
    else setW((k) => ({ ...k, exif: { ...k.exif, ...patch } }));
  };

  // ---- staging (Qt) ----
  const onStaged = (newPaths: string[], names: string[]) => {
    if (!batch) {
      let ps = newPaths, ns = names;
      if (u && !c) {
        const first = aa(names[0]);
        setO(first);
        // keep only files matching the first file's type
        const keep = names.map((n, i) => (aa(n) === first ? i : -1)).filter((i) => i >= 0);
        ps = keep.map((i) => newPaths[i]);
        ns = keep.map((i) => names[i]);
        if (keep.length < names.length) flash(`Skipped ${names.length - keep.length} file(s) of a different type.`);
      }
      setBatch({ paths: ps, names: ns });
      setOutNames(ns.map((n) => jl(n)));
      setHe(0);
    } else {
      const type = O;
      const keep = names.map((n, i) => (c || aa(n) === type ? i : -1)).filter((i) => i >= 0);
      const addP = keep.map((i) => newPaths[i]);
      const addN = keep.map((i) => names[i]);
      if (keep.length < names.length) flash(`Skipped ${names.length - keep.length} file(s) of a different type.`);
      setBatch({ paths: [...batch.paths, ...addP], names: [...batch.names, ...addN] });
      setOutNames((prev) => [...prev, ...addN.map((n) => jl(n))]);
    }
  };

  const removeAt = (k: number) => {
    if (!batch) return;
    const paths2 = batch.paths.filter((_, i) => i !== k);
    const names2 = batch.names.filter((_, i) => i !== k);
    setOutNames((prev) => prev.filter((_, i) => i !== k));
    if (paths2.length === 0) { setBatch(null); setOutNames([]); }
    else setBatch({ paths: paths2, names: names2 });
    setHe((h) => Math.max(0, Math.min(h, paths2.length - 1)));
  };

  const clearAll = () => { setBatch(null); setOutNames([]); setE(""); setHe(0); };

  // ---- concurrency ----
  const onConcurrency = (v: number) => { setBe(v); T("set_concurrency", { value: v }).catch(() => {}); };

  // ---- trim setter (Fl) forces trim_start/end true ----
  const setTrim = (v: number) => {
    if (c) ie((k) => ({ ...k, trim_secs: v, trim_start: true, trim_end: true }));
    else setL((k) => ({ ...k, trim_secs: v, trim_start: true, trim_end: true }));
  };

  // ---- change / reset output dir ----
  const changeDir = async () => {
    if (!isTauri()) { flash("Open the desktop app to choose a folder."); return; }
    try {
      const dir = await openDialog({ directory: true, multiple: false });
      if (typeof dir === "string") setB(dir);
    } catch (e) {
      flash(String(e));
    }
  };

  // ---- run (I) ----
  const run = async () => {
    if (!batch || ae) return;
    setAe(true);
    try {
      const baseDir = B || defaultOut || paths?.outputs || null;
      // batch-folder name uses the (extension-stripped) output name de[0], not the raw filename
      const xe = baseDir ? Wg(baseDir, qi(E || outNames[0] || "spoof")) : null;
      for (let Pe = 0; Pe < batch.paths.length; Pe++) {
        const tt = u ? qi(outNames[Pe] || jl(batch.names[Pe])) : null;
        await sr(batch.paths[Pe], tt, xe);
      }
      reloadJobs();
    } catch (e) {
      flash(String(e));
    } finally {
      setAe(false);
    }
  };

  // dispatcher sr
  const sr = async (inputPath: string, baseName: string | null, outputDir: string | null) => {
    if (c) return T("create_reddit_spoof_job", { platform: l, inputPath, config: ee, baseName, outputDir });
    if (u && O === "image") return T("create_image_spoof_job", { platform: l, inputPath, config: W, baseName, outputDir });
    if (u) return T("create_spoof_job", { platform: l, inputPath, config: L, baseName, outputDir });
    return T("create_processing_job", { platform: l, inputPath });
  };

  const activeJob = jobs.find((j) => j.status === "processing" || j.status === "queued");
  const pageOutputs = outputs.slice(outPage * PAGE_SIZE, outPage * PAGE_SIZE + PAGE_SIZE);
  const unreviewed = outputs.filter((o) => !o.reviewed);

  const iosDevices = devices.filter((d) => d.os === "ios");
  const androidDevices = devices.filter((d) => d.os === "android");

  const P = batch?.paths.length ?? 0;

  return (
    <div className="page">
      <div className="page-header">
        <div className="eyebrow" style={{ color: s.color }}>
          <span className="nav-platform-dot" style={{ color: s.color, background: s.color, display: "inline-block", marginRight: 6 }} />
          {s.label}
        </div>
        <h1>{s.label} <span className="badge badge-accent">{u ? "Spoofer" : "Coming soon"}</span></h1>
        <p className="page-intro">
          {c
            ? "Uniquify your media so each upload looks freshly captured."
            : "Auto-detects video or image from your first drop and spoofs every copy."}
        </p>
      </div>

      {/* drop zone */}
      <div className="card">
        <DropZone onStaged={onStaged} extensions={Ar} label={Ur} multiple disabled={ae} />
        {batch && (
          <div className="staged-preview" style={{ marginTop: "1rem" }}>
            <button className="staged-nav" disabled={he <= 0} onClick={() => setHe((h) => h - 1)}>◀</button>
            <div className="staged-stack">
              <div className="staged-stack-front">
                <StagedPreview path={batch.paths[he]} name={batch.names[he]} />
                <button className="staged-remove" onClick={() => removeAt(he)}>✕</button>
              </div>
            </div>
            <button className="staged-nav" disabled={he >= P - 1} onClick={() => setHe((h) => h + 1)}>▶</button>
          </div>
        )}
        {batch && <div className="staged-caption">{he + 1} / {P}</div>}
      </div>

      {/* controls */}
      {batch && (
        <div className="card">
          <div className="card-title">Spoof settings</div>

          {/* Strength (video for IG, always for reddit/twitter) */}
          {((u && !c && O === "video") || c) && (
            <div className="control-row">
              <div className="control-label">
                <span className="lbl" title={c ? "How hard we transform — scales crop, colour, noise and the dHash target" : undefined}>Strength</span>
              </div>
              <SegToggle
                className="strength-toggle"
                options={Pd.map((p) => ({ id: p.id, label: p.label, title: p.hint }))}
                value={(c ? ee.strength : L.strength)}
                disabled={ae}
                onChange={(id) => c ? ie((k) => ({ ...k, strength: id as any })) : setL((k) => ({ ...k, strength: id as any }))}
              />
            </div>
          )}

          {/* Speed shift (IG video only) */}
          {u && !c && O === "video" && (
            <div className="control-row">
              <div className="control-label">
                <span className="lbl" title="Apply a small speed change to each output">Speed shift</span>
              </div>
              <SegToggle
                options={[{ id: "off", label: "Off" }, { id: "on", label: "On" }]}
                value={L.speed_spoof ? "on" : "off"}
                disabled={ae}
                onChange={(id) => setL((k) => ({ ...k, speed_spoof: id === "on" }))}
              />
            </div>
          )}

          {/* Copies / Variants */}
          <div className="control-row">
            <div className="control-label"><span className="lbl">{gn}</span></div>
            <Stepper
              value={c ? ee.copies : O === "image" ? W.copies : L.outputs_per_source}
              min={1}
              max={maxCopies(O === "image", c)}
              disabled={ae}
              onChange={(v) => c ? ie((k) => ({ ...k, copies: v })) : O === "image" ? setW((k) => ({ ...k, copies: v })) : setL((k) => ({ ...k, outputs_per_source: v }))}
            />
          </div>

          {/* Parallel spoofs */}
          <div className="control-row">
            <div className="control-label"><span className="lbl">Parallel spoofs</span></div>
            <Stepper value={be} min={1} max={Qv} disabled={ae} onChange={onConcurrency} />
          </div>

          {/* Trim ends (IG video, or reddit/twitter when a video is staged) */}
          {((u && !c && O === "video") || (c && Il)) && (
            <div className="control-row">
              <div className="control-label">
                <span className="lbl" title="Trim a little off the start & end of each video">Trim ends</span>
              </div>
              <Stepper
                value={c ? ee.trim_secs : L.trim_secs}
                min={0}
                max={1}
                step={0.1}
                disabled={ae}
                format={(v) => (v <= 0 ? "Off" : `${v.toFixed(1)}s`)}
                onChange={setTrim}
              />
            </div>
          )}

          {/* Mirror — IG: percentage range; reddit/twitter: Off/On toggle */}
          {u && !c && O === "video" && (
            <div className="control-row">
              <div className="control-label">
                <span className="lbl" title="Share of outputs that get horizontally mirrored">Mirror · {L.mirror_pct}%</span>
              </div>
              <input type="range" min={0} max={100} step={5} value={L.mirror_pct} disabled={ae}
                onChange={(e) => setL((k) => ({ ...k, mirror_pct: parseInt(e.target.value) }))} />
            </div>
          )}
          {c && (
            <div className="control-row">
              <div className="control-label">
                <span className="lbl" title="Mirrors the image horizontally — strongest against dHash, but visibly flipped">Mirror</span>
              </div>
              <SegToggle options={[{ id: "off", label: "Off" }, { id: "on", label: "On" }]} value={ee.mirror ? "on" : "off"} disabled={ae}
                onChange={(id) => ie((k) => ({ ...k, mirror: id === "on" }))} />
            </div>
          )}

          {/* EXIF camera */}
          {it && (
            <div className="control-row" style={{ flexWrap: "wrap" }}>
              <div className="control-label">
                <span className="lbl" title="Fake camera EXIF written into every spoofed photo — make, model and a fresh capture date">EXIF camera</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem", alignItems: "flex-end" }}>
                <SegToggle
                  options={exifButtons.map((b) => ({ id: b.key, label: b.label, title: b.title }))}
                  value={exifCfg.mode === "off" ? "off" : exifCfg.pool === "ios" ? "ios" : exifCfg.pool === "android" ? "android" : "random"}
                  disabled={ae}
                  onChange={(id) => { const b = exifButtons.find((x) => x.key === id)!; setExif(b.value); }}
                />
                {(exifCfg.pool === "ios" || exifCfg.pool === "android") && (
                  <select
                    className="exif-model-select"
                    value={exifCfg.device_id ?? ""}
                    disabled={ae}
                    onChange={(e) => {
                      const id = e.target.value;
                      if (!id) setExif({ mode: "random", pool: exifCfg.pool, device_id: null });
                      else setExif({ mode: "device", pool: exifCfg.pool, device_id: id });
                    }}
                  >
                    <option value="">{exifCfg.pool === "ios" ? "Any iPhone (random)" : "Any Android (random)"}</option>
                    {(exifCfg.pool === "ios" ? iosDevices : androidDevices).map((d) => (
                      <option key={d.id} value={d.id}>{d.label}</option>
                    ))}
                  </select>
                )}
              </div>
            </div>
          )}
        </div>
      )}

      {/* naming + output */}
      {batch && (
        <div className="card">
          <div className="card-title">Output</div>
          <label className="field-label">Output name</label>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <input
              className="text-input"
              value={outNames[he] ?? ""}
              placeholder={jl(batch.names[he] || "")}
              onChange={(e) => setOutNames((prev) => prev.map((n, i) => (i === he ? e.target.value : n)))}
            />
            <button className="btn ghost small" onClick={() => setOutNames((prev) => prev.map(() => outNames[he] ?? ""))}>Apply to all</button>
          </div>
          <div className="control-hint" style={{ marginTop: 4 }}>Saved as <code>{qi(outNames[he] || jl(batch.names[he] || ""))}_001…</code></div>

          <label className="field-label" style={{ marginTop: "0.75rem" }}>Batch folder name</label>
          <input className="text-input" value={E} placeholder={qi(outNames[0] || "spoof")} onChange={(e) => setE(e.target.value)} />

          <div className="control-row" style={{ marginTop: "0.75rem" }}>
            <div className="control-label">
              <span className="lbl">Output folder</span>
              <span className="control-hint">{basename(B || defaultOut || paths?.outputs || "app folder")}</span>
            </div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button className="btn ghost small" onClick={changeDir}>Change…</button>
              {B && <button className="btn ghost small" onClick={() => setB(null)}>Reset</button>}
            </div>
          </div>
        </div>
      )}

      {/* run */}
      {batch && (
        <div className="card" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div className="muted">{P * Re} output(s) total</div>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button className="btn ghost" onClick={clearAll} disabled={ae}>Clear</button>
            <button className="btn primary" onClick={run} disabled={ae}>
              {ae ? "Starting…" : `Start spoofing ${P} file(s)`}
            </button>
          </div>
        </div>
      )}

      {/* active job */}
      {activeJob && (
        <div className="card">
          <div className="card-head">
            <span className="status-pill live">Active job</span>
            <button className="btn danger small" onClick={() => T("cancel_spoof_job", { jobId: activeJob.id }).then(reloadJobs)}>Stop</button>
          </div>
          <div className="progress-track"><div className="progress-bar" style={{ width: `${activeJob.progress}%` }} /></div>
        </div>
      )}

      {/* recent jobs */}
      {jobs.length > 0 && (
        <div className="card">
          <div className="card-head">
            <div className="card-title">Recent jobs</div>
            <button className="btn ghost small" onClick={() => T("clear_jobs", { platform: l }).then(reloadJobs)}>Clear</button>
          </div>
          {jobs.slice(0, 8).map((j) => (
            <div className="list-row" key={j.id}>
              <span className={`dot`} style={{ background: j.status === "completed" ? "#3ddc84" : j.status === "failed" ? "#ff5a5f" : "#f59e0b" }} />
              <div className="row-main">
                <div className="row-name">{basename(j.input_path)}</div>
                <div className="row-sub">{j.status} · {j.progress}%{j.error ? ` · ${j.error}` : ""}</div>
              </div>
              <button className="icon-btn" onClick={() => T("delete_job", { jobId: j.id }).then(reloadJobs)}>🗑</button>
            </div>
          ))}
        </div>
      )}

      {/* recent outputs */}
      {outputs.length > 0 && (
        <div className="card">
          <div className="card-head">
            <div className="card-title">Recent outputs</div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              {unreviewed.length > 0 && <button className="btn primary small" onClick={() => setReview(true)}>Review ({unreviewed.length})</button>}
              <button className="btn ghost small" onClick={() => T("open_outputs_folder", { platform: l })}>Open folder</button>
              <button className="btn ghost small" onClick={() => T("clear_outputs", { platform: l }).then(reloadOutputs)}>Clear list</button>
            </div>
          </div>
          {pageOutputs.map((o) => (
            <div className="list-row" key={o.id}>
              <img className="output-thumb" src={convertFileSrc(o.file_path)} alt="" />
              <div className="row-main">
                <div className="row-name">{o.file_name}</div>
                <div className="row-sub">{o.reviewed ? "reviewed" : "new"}</div>
              </div>
              <button className="icon-btn" onClick={() => T("open_output_file", { filePath: o.file_path })}>Open</button>
              <button className="icon-btn output-delete" onClick={() => T("delete_output", { outputId: o.id }).then(reloadOutputs)}>🗑</button>
            </div>
          ))}
          {outputs.length > PAGE_SIZE && (
            <div className="pager">
              <button disabled={outPage <= 0} onClick={() => setOutPage((p) => p - 1)}>← Prev</button>
              <span>Page {outPage + 1} / {Math.ceil(outputs.length / PAGE_SIZE)}</span>
              <button disabled={(outPage + 1) * PAGE_SIZE >= outputs.length} onClick={() => setOutPage((p) => p + 1)}>Next →</button>
            </div>
          )}
        </div>
      )}

      {review && (
        <ReviewModal
          items={unreviewed}
          onClose={() => { setReview(false); reloadOutputs(); }}
          openItem={(o) => openPath(o.file_path)}
          onDeleted={reloadOutputs}
          deleteItem={(id) => T("delete_output", { outputId: id }).then(() => {})}
          markReviewed={(ids) => T("mark_outputs_reviewed", { outputIds: ids }).then(() => {})}
        />
      )}

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}

function StagedPreview({ path, name }: { path: string; name: string }) {
  const src = convertFileSrc(path);
  if (aa(name) === "video") return <video src={src} controls muted playsInline preload="metadata" />;
  return <img src={src} alt={name} />;
}
