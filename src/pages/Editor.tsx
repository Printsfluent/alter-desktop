// Ud({mode}) — the shared reel editor. mode="captions" renders the auto-subtitles
// pipeline; mode="snapchat" renders the stripped-down static text-bar tool. Both
// share the drop zone, live phone-frame preview, export button, and outputs panel.
import { useEffect, useRef, useState } from "react";
import { T, listen, convertFileSrc } from "../lib/tauri";
import { ds, qv, eg, ng, rg, bf, lg, Id, PAGE_SIZE, rr } from "../lib/constants";
import { ir, basename } from "../lib/helpers";
import type {
  SubtitleConfig, VideoTemplate, VideoJobRecord, VideoOutputRecord,
  CaptionResult, CaptionSegment, CaptionWord, ProbeResult,
} from "../lib/types";
import { SegToggle } from "../components/SegToggle";
import { NumStepper } from "../components/NumStepper";
import { DropZone } from "../components/DropZone";

// Preview scaling constants (@227490): va=360px preview width, ma/Fd = export
// dimensions, Ad = 1/3 caption font scale applied only in the captions preview.
const va = 360;
const ma = ds.export_width; // 1080
const Fd = ds.export_height; // 1920
const Ad = va / ma; // 0.3333

// renderOptions builders — both spread ds and override subtitle/overlay fields only.
const ig = (subtitlesPath: string) => ({
  ...ds,
  subtitle_enabled: true,
  subtitles_path: subtitlesPath,
  caption_text: null,
  caption_style: qv,
  overlay_preset: "subtitles",
});
const sg = (text: string, overlay_y = 0.5) => ({
  ...ds,
  subtitle_enabled: true,
  caption_text: text,
  caption_style: eg,
  overlay_preset: "snapchat",
  subtitles_path: null,
  overlay_y,
});

// Og — active-preset test: matches only mode/font/primary/highlight (colors
// case-insensitive)/use_box/uppercase. Size, outline, max_words, position ignored.
function Og(cur: SubtitleConfig, p: SubtitleConfig): boolean {
  return (
    cur.mode === p.mode &&
    cur.font === p.font &&
    cur.primary_color.toLowerCase() === p.primary_color.toLowerCase() &&
    cur.highlight_color.toLowerCase() === p.highlight_color.toLowerCase() &&
    cur.use_box === p.use_box &&
    cur.uppercase === p.uppercase
  );
}

// Ig — reconcile edited transcript segments back onto word timings. Words whose
// start falls within a segment (±0.05s) keep their timing; otherwise the segment
// is split into equal slices (min segment length 0.1s / word count).
function Ig(segments: CaptionSegment[], words: CaptionWord[]): CaptionWord[] {
  const out: CaptionWord[] = [];
  for (const seg of segments) {
    const tokens = seg.text.trim().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) continue;
    const inSeg = words.filter((w) => w.start >= seg.start - 0.05 && w.start <= seg.end + 0.05);
    if (inSeg.length === tokens.length) {
      tokens.forEach((t, i) => out.push({ word: t, start: inSeg[i].start, end: inSeg[i].end }));
    } else {
      const slice = Math.max(seg.end - seg.start, 0.1) / tokens.length;
      tokens.forEach((t, i) =>
        out.push({ word: t, start: seg.start + i * slice, end: seg.start + (i + 1) * slice }),
      );
    }
  }
  return out;
}

// zd — classify a video job by its render_options.overlay_preset for the type badge.
function classify(job: VideoJobRecord): string {
  try {
    const ro = job.render_options ? JSON.parse(job.render_options) : null;
    if (ro && ro.overlay_preset === "snapchat") return "snapchat";
    if (ro && (ro.overlay_preset === "subtitles" || ro.subtitles_path)) return "subtitles";
  } catch {
    /* ignore malformed render_options */
  }
  return "spoof";
}

interface JobProgress {
  status?: string;
}

export function Editor({ mode }: { mode: "captions" | "snapchat" }) {
  const captions = mode === "captions";

  // live caption-burn config (captions mode)
  const [V, setV] = useState<SubtitleConfig>(ng);
  const [J, setJ] = useState("auto"); // spoken language
  const [ne, setNe] = useState(0.5); // snapchat overlay_y
  const [ve, setVe] = useState(""); // snapchat overlay text
  const [playT, setPlayT] = useState(0); // preview playhead
  const [presetOpen, setPresetOpen] = useState(true);

  const [f, setF] = useState<string | null>(null); // staged video path
  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [L, setL] = useState<CaptionResult | null>(null); // caption result
  const [Ee, setEe] = useState<CaptionSegment[]>([]); // editable transcript rows

  const [templates, setTemplates] = useState<VideoTemplate[]>([]);
  const [jobs, setJobs] = useState<VideoJobRecord[]>([]);
  const [outputs, setOutputs] = useState<VideoOutputRecord[]>([]);
  const [outPage, setOutPage] = useState(0);

  const [busy, setBusy] = useState(false);
  const [transcribing, setTranscribing] = useState(false);
  const [statusText, setStatusText] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const videoRef = useRef<HTMLVideoElement | null>(null);

  const flash = (m: string) => {
    setToast(m);
    window.setTimeout(() => setToast(null), 3000);
  };

  const patch = (p: Partial<SubtitleConfig>) => setV((c) => ({ ...c, ...p }));

  // ---- loads ----
  const reloadJobs = () => T<VideoJobRecord[]>("list_video_jobs").then(setJobs).catch(() => {});
  const reloadOutputs = () => T<VideoOutputRecord[]>("list_video_outputs").then(setOutputs).catch(() => {});
  const reloadTemplates = () =>
    T<VideoTemplate[]>("list_video_templates")
      .then((list) => setTemplates(list.filter((t) => t.subtitle_config)))
      .catch(() => {});

  useEffect(() => {
    reloadJobs();
    reloadOutputs();
    if (captions) reloadTemplates();
  }, [mode]); // eslint-disable-line

  useEffect(() => {
    const uns: (() => void)[] = [];
    listen<string>("caption-status", (p) => setStatusText(typeof p === "string" ? p : String(p))).then((fn) =>
      uns.push(fn),
    );
    listen<JobProgress>("video-job-progress", (p) => {
      reloadJobs();
      if (p && p.status === "completed") reloadOutputs();
    }).then((fn) => uns.push(fn));
    return () => uns.forEach((fn) => fn());
  }, [mode]); // eslint-disable-line

  // ---- staging ----
  const onStaged = (paths: string[]) => {
    const p = paths[0];
    if (!p) return;
    setF(p);
    setProbe(null);
    setL(null);
    setEe([]);
    setPlayT(0);
    T<ProbeResult>("probe_video_file", { path: p }).then(setProbe).catch(() => setProbe(null));
  };

  // ---- transcript pipeline ----
  const generate = async () => {
    if (!f) {
      flash("Upload video first");
      return;
    }
    setTranscribing(true);
    setStatusText(null);
    try {
      const res = await T<CaptionResult>("generate_auto_captions", {
        videoPath: f,
        language: J === "auto" ? null : J,
      });
      setL(res);
      setEe(res.segments.map((s) => ({ ...s })));
    } catch (e) {
      flash(String(e));
    } finally {
      setTranscribing(false);
    }
  };

  const cancelCaption = () => {
    T("cancel_caption_task").catch(() => {});
  };

  const editSeg = (i: number, text: string) =>
    setEe((prev) => prev.map((s, j) => (j === i ? { ...s, text } : s)));

  const seekTo = (t: number) => {
    if (videoRef.current) videoRef.current.currentTime = t + 0.01;
    setPlayT(t + 0.01);
  };

  // ---- templates ----
  // chip apply = full restore incl. the template's saved position (pe(t.subtitle_config));
  // only *presets* preserve the current position.
  const applyTemplate = (t: VideoTemplate) => {
    if (t.subtitle_config) setV(t.subtitle_config);
  };

  const delTemplate = (t: VideoTemplate) => {
    setTemplates((prev) => prev.filter((x) => x.id !== t.id));
    T("delete_video_template", { templateId: t.id }).then(reloadTemplates).catch(reloadTemplates);
  };

  const saveTemplate = () => {
    const name = window.prompt("Template name");
    if (!name || !name.trim()) return;
    T("create_video_template", {
      input: { name: name.trim(), subtitle_enabled: true, subtitle_config: V },
    })
      .then(reloadTemplates)
      .catch((e) => flash(String(e)));
  };

  // ---- export ----
  const onExport = async () => {
    if (busy) return;
    if (!f) {
      flash("Upload video first");
      return;
    }
    if (captions) {
      if (!L) {
        flash("Run auto subtitles first");
        return;
      }
      setBusy(true);
      try {
        const words = Ig(Ee, L.words);
        const assPath = await T<string>("build_subtitle_track", {
          caption: { srt_path: L.srt_path, segments: Ee, words },
          config: V,
          width: ma,
          height: Fd,
        });
        await T("create_video_job", {
          mode: "quick",
          inputPath: f,
          templateId: null,
          renderOptions: ig(assPath),
        });
        await reloadJobs();
      } catch (e) {
        flash(String(e));
      } finally {
        setBusy(false);
      }
    } else {
      if (!ve.trim()) {
        flash("Type overlay text");
        return;
      }
      setBusy(true);
      try {
        await T("create_video_job", {
          mode: "quick",
          inputPath: f,
          templateId: null,
          renderOptions: sg(ve.trim(), ne),
        });
        await reloadJobs();
      } catch (e) {
        flash(String(e));
      } finally {
        setBusy(false);
      }
    }
  };

  const delOutput = (id: string) => {
    setOutputs((prev) => prev.filter((o) => o.id !== id));
    T("delete_video_output", { outputId: id }).then(reloadOutputs).catch(reloadOutputs);
  };

  // ---- derived ----
  const activeJob = jobs.find((j) => j.status === "processing" || j.status === "queued");
  const pageOutputs = outputs.slice(outPage * PAGE_SIZE, outPage * PAGE_SIZE + PAGE_SIZE);
  const previewLabel = !captions ? "Preview (Snapchat text)" : f ? "Preview (with subtitles)" : "Live preview";

  // caption preview tokens at the playhead
  const capPrev = (() => {
    const words = L && L.words.length ? L.words : null;
    const caps = (arr: string[]) => (V.uppercase ? arr.map((x) => x.toUpperCase()) : arr);
    if (!words) {
      if (V.mode === "word") return { tokens: caps(["Caption"]), active: 0 };
      return { tokens: caps(["This", "is", "your", "caption"]), active: 1 };
    }
    let ai = 0;
    for (let i = 0; i < words.length; i++) {
      if (words[i].start <= playT) ai = i;
      else break;
    }
    if (V.mode === "word") return { tokens: caps([words[ai].word]), active: 0 };
    const mw = Math.max(1, V.max_words);
    const ls = Math.floor(ai / mw) * mw;
    return { tokens: caps(words.slice(ls, ls + mw).map((w) => w.word)), active: ai - ls };
  })();

  return (
    <div className="page">
      <div className="page-header">
        <div className="eyebrow">Video editing</div>
        <h1>
          {captions ? "Auto subtitles" : "Snapchat text"} <span className="badge badge-accent">Preset</span>
        </h1>
        <p className="page-intro">
          {captions
            ? "Drop a clip and burn in animated subtitles generated from the speech."
            : "Drop a clip and burn in a Snapchat-style static text bar."}
        </p>
      </div>

      {/* drop zone */}
      <div className="card">
        <DropZone onStaged={onStaged} extensions={rr} label="Drop a video to edit" videoOnly disabled={busy} />
        {f && (
          <div className="control-hint" style={{ marginTop: "0.6rem" }}>
            {basename(f)}
            {probe ? ` · ${probe.width}×${probe.height} · ${probe.duration_secs.toFixed(1)}s` : ""}
          </div>
        )}
      </div>

      {/* studio: controls + preview */}
      <div className="card">
        <div className="subtitle-studio">
          <div className="subtitle-controls">
            {captions ? (
              <>
                {/* style presets */}
                <button
                  type="button"
                  className="btn ghost small"
                  style={{ justifyContent: "flex-start" }}
                  onClick={() => setPresetOpen((o) => !o)}
                >
                  {presetOpen ? "▾" : "▸"} Style preset
                </button>
                {presetOpen && (
                  <>
                    <div className="control-hint">Pick a look, then fine-tune below.</div>
                    <div className="subtitle-preset-grid">
                      {rg.map((preset) => {
                        const cfg = preset.config;
                        const isWord = cfg.mode === "word";
                        const toks = isWord ? ["Quick"] : ["The", "quick"];
                        return (
                          <button
                            key={preset.id}
                            type="button"
                            className={`subtitle-preset-card${Og(V, cfg) ? " active" : ""}`}
                            onClick={() => setV((cur) => ({ ...cfg, position: cur.position }))}
                          >
                            <div style={{ fontFamily: bf(cfg.font), fontWeight: 700, fontSize: 13, lineHeight: 1.1 }}>
                              {toks.map((t, i) => (
                                <span
                                  key={i}
                                  style={{
                                    color: !isWord && i === 1 ? cfg.highlight_color : cfg.primary_color,
                                  }}
                                >
                                  {cfg.uppercase ? t.toUpperCase() : t}{" "}
                                </span>
                              ))}
                            </div>
                            <div className="subtitle-preset-name">{preset.label}</div>
                          </button>
                        );
                      })}
                    </div>
                  </>
                )}

                {/* saved templates */}
                <div className="field-label" style={{ marginTop: "0.5rem" }}>
                  Saved templates
                </div>
                {templates.length > 0 && (
                  <div className="template-chips">
                    {templates.map((t) => (
                      <span className="template-chip" key={t.id}>
                        <button className="template-chip-apply" onClick={() => applyTemplate(t)}>
                          {t.name}
                        </button>
                        <button className="template-chip-del" aria-label="Delete template" onClick={() => delTemplate(t)}>
                          ×
                        </button>
                      </span>
                    ))}
                  </div>
                )}
                <button className="btn ghost small save-template-btn span-2" onClick={saveTemplate}>
                  Save current as template
                </button>

                {/* subtitle style */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Subtitle style</span>
                  </div>
                  <SegToggle
                    options={[
                      { id: "highlight", label: "Highlight word", title: "Whole line stays on screen, the spoken word lights up." },
                      { id: "word", label: "Word for word", title: "One word at a time, synced to speech." },
                    ]}
                    value={V.mode}
                    onChange={(id) => patch({ mode: id as "highlight" | "word" })}
                  />
                </div>

                {/* size */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Size</span>
                  </div>
                  <NumStepper value={V.font_size} min={32} max={120} step={1} ariaLabel="Font size" onCommit={(v) => patch({ font_size: v })} />
                </div>

                {/* outline */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Outline</span>
                  </div>
                  <NumStepper value={V.outline} min={0} max={12} step={0.5} ariaLabel="Outline" onCommit={(v) => patch({ outline: v })} />
                </div>

                {/* vertical position */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Vertical position</span>
                  </div>
                  <NumStepper
                    value={Math.round(V.position * 100)}
                    min={10}
                    max={95}
                    step={1}
                    suffix="%"
                    ariaLabel="Vertical position"
                    onCommit={(v) => patch({ position: v / 100 })}
                  />
                </div>

                {/* words per line — highlight only */}
                {V.mode === "highlight" && (
                  <div className="control-row">
                    <div className="control-label">
                      <span className="lbl">Words per line</span>
                    </div>
                    <NumStepper value={V.max_words} min={1} max={8} step={1} ariaLabel="Words per line" onCommit={(v) => patch({ max_words: v })} />
                  </div>
                )}

                {/* text color */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Text color</span>
                  </div>
                  <input type="color" value={V.primary_color} onChange={(e) => patch({ primary_color: e.target.value })} />
                </div>

                {/* highlight color — highlight only */}
                {V.mode === "highlight" && (
                  <div className="control-row">
                    <div className="control-label">
                      <span className="lbl">Highlight color</span>
                    </div>
                    <input type="color" value={V.highlight_color} onChange={(e) => patch({ highlight_color: e.target.value })} />
                  </div>
                )}

                {/* box */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Box behind text</span>
                  </div>
                  <input type="checkbox" checked={V.use_box} onChange={(e) => patch({ use_box: e.target.checked })} />
                </div>

                {/* uppercase */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">ALL CAPS</span>
                  </div>
                  <input type="checkbox" checked={V.uppercase} onChange={(e) => patch({ uppercase: e.target.checked })} />
                </div>

                {/* language */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Spoken language</span>
                  </div>
                  <select id="caption-language-select" className="text-input" value={J} onChange={(e) => setJ(e.target.value)}>
                    {lg.map((o) => (
                      <option key={o.code} value={o.code}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                </div>

                {/* pipeline */}
                <div className="control-row" style={{ gap: "0.5rem" }}>
                  <button className="btn primary" disabled={transcribing || !f} onClick={generate}>
                    {transcribing ? "Transcribing…" : L ? "Re-generate subtitles" : "1 · Generate subtitles"}
                  </button>
                  {transcribing && (
                    <button className="btn danger small" onClick={cancelCaption}>
                      Stop
                    </button>
                  )}
                </div>
                {statusText && <div className="control-hint">{statusText}</div>}

                {/* transcript editor */}
                {L && (
                  <div>
                    <div className="field-label" style={{ marginTop: "0.5rem" }}>
                      Transcript — fix any mistakes
                    </div>
                    <div className="control-hint">click a timestamp to jump in the preview</div>
                    {Ee.map((seg, i) => (
                      <div className="transcript-row" key={i}>
                        <button className="transcript-time" onClick={() => seekTo(seg.start)}>
                          {ir(seg.start)}
                        </button>
                        <input className="text-input transcript-input" value={seg.text} onChange={(e) => editSeg(i, e.target.value)} />
                      </div>
                    ))}
                  </div>
                )}
              </>
            ) : (
              <>
                {/* snapchat — overlay text */}
                <label className="field-label" htmlFor="snapchat-text">
                  Overlay text
                </label>
                <textarea
                  id="snapchat-text"
                  className="text-input"
                  rows={3}
                  value={ve}
                  onChange={(e) => setVe(e.target.value)}
                  placeholder="Text on video..."
                />
                <div className="control-hint">One static text bar burned over the whole clip.</div>

                {/* snapchat — vertical position */}
                <div className="control-row">
                  <div className="control-label">
                    <span className="lbl">Vertical position</span>
                  </div>
                  <NumStepper
                    value={Math.round(ne * 100)}
                    min={0}
                    max={100}
                    step={1}
                    suffix="%"
                    ariaLabel="Vertical position"
                    onCommit={(v) => setNe(v / 100)}
                  />
                </div>
              </>
            )}

            {/* export */}
            <button
              className="btn primary"
              style={{ marginTop: "0.75rem" }}
              disabled={busy || (captions && !L)}
              onClick={onExport}
            >
              {busy ? "Exporting..." : captions ? "2 · Export video" : "Export video"}
            </button>
          </div>

          {/* preview */}
          <div className="phone-frame">
            <div className="field-label">{previewLabel}</div>
            <div className="subtitle-preview">
              {f && (
                <video
                  ref={videoRef}
                  key={f}
                  src={convertFileSrc(f)}
                  preload="metadata"
                  muted
                  playsInline
                  controls
                  onTimeUpdate={(e) => setPlayT((e.target as HTMLVideoElement).currentTime)}
                  style={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "cover" }}
                />
              )}
              {!captions ? (
                <div className="snapchat-caption-bar" style={{ top: `${ne * 100}%` }}>
                  <p className="snapchat-preview-text">{ve.trim() || "Your text here"}</p>
                </div>
              ) : (
                <div
                  className="caption-preview-text"
                  style={{
                    top: `${V.position * 100}%`,
                    fontFamily: bf(V.font),
                    fontSize: `${V.font_size * Ad}px`,
                    WebkitTextStroke: `${V.outline * Ad}px #000`,
                    background: "rgba(0,0,0,0.7)",
                  }}
                >
                  {capPrev.tokens.map((t, i) => (
                    <span
                      key={i}
                      style={{
                        color: V.mode === "highlight" && i === capPrev.active ? V.highlight_color : V.primary_color,
                      }}
                    >
                      {t}
                      {i < capPrev.tokens.length - 1 ? " " : ""}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* active job */}
      {activeJob && (
        <div className="card">
          <div className="card-head">
            <span className="status-pill live">Rendering...</span>
            <span className={`badge ${Id[classify(activeJob)].className}`}>{Id[classify(activeJob)].label}</span>
            <button
              className="btn danger small"
              onClick={() => T("cancel_video_job", { jobId: activeJob.id }).then(reloadJobs).catch(() => {})}
            >
              Stop
            </button>
          </div>
          <div className="progress-track">
            <div className="progress-bar" style={{ width: `${activeJob.progress}%` }} />
          </div>
        </div>
      )}

      {/* recent exports */}
      {outputs.length > 0 && (
        <div className="card">
          <div className="card-head">
            <div className="card-title">Recent exports</div>
            <button className="btn ghost small" onClick={() => T("open_video_outputs_folder").catch(() => {})}>
              Open folder
            </button>
          </div>
          {pageOutputs.map((o) => (
            <div className="list-row" key={o.id}>
              <video className="output-thumb" src={`${convertFileSrc(o.file_path)}#t=0.2`} muted preload="metadata" />
              <div className="row-main">
                <div className="row-name">{o.file_name}</div>
                <div className="row-sub">{o.template_name || "video.mp4"}</div>
              </div>
              <button className="icon-btn" onClick={() => T("open_video_output_file", { filePath: o.file_path }).catch(() => {})}>
                Open
              </button>
              <button className="icon-btn" onClick={() => delOutput(o.id)}>
                🗑
              </button>
            </div>
          ))}
          {outputs.length > PAGE_SIZE && (
            <div className="pager">
              <button disabled={outPage <= 0} onClick={() => setOutPage((p) => p - 1)}>
                ← Prev
              </button>
              <span>
                Page {outPage + 1} / {Math.ceil(outputs.length / PAGE_SIZE)}
              </span>
              <button disabled={(outPage + 1) * PAGE_SIZE >= outputs.length} onClick={() => setOutPage((p) => p + 1)}>
                Next →
              </button>
            </div>
          )}
        </div>
      )}

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
