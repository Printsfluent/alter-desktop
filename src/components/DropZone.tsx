// Pa — shared drop zone. Browse via the dialog plugin + OS drag-drop.
// Validates extensions, then (when stage) copies each file through
// stage_file_upload before returning staged paths + original names.
import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { T, isTauri } from "../lib/tauri";
import { extname, basename } from "../lib/helpers";

export function DropZone(props: {
  onStaged: (paths: string[], names: string[]) => void;
  extensions: string[];
  label?: string;
  disabled?: boolean;
  multiple?: boolean;
  videoOnly?: boolean;
  imageOnly?: boolean;
  stage?: boolean;
}) {
  const {
    onStaged, extensions, label = "Drop files here", disabled,
    multiple = false, videoOnly = false, imageOnly = false, stage = true,
  } = props;
  const [drag, setDrag] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const subLabel = videoOnly ? "Video files" : imageOnly ? "Image files" : "Image or video";

  const handlePaths = async (paths: string[]) => {
    setErr(null);
    const list = multiple ? paths : paths.slice(0, 1);
    const good = list.filter((p) => extensions.includes(extname(p)));
    if (good.length === 0) {
      if (videoOnly) setErr("Only video files supported (mp4, mov, mkv, webm, avi)");
      else if (imageOnly) setErr("Only image files supported (jpg, png, webp, heic)");
      else setErr(`Supported types: ${extensions.join(", ")}`);
      return;
    }
    try {
      setBusy(true);
      const names = good.map((p) => basename(p));
      let out = good;
      if (stage) {
        out = [];
        for (const p of good) {
          const staged = await T<string>("stage_file_upload", { sourcePath: p });
          out.push(staged);
        }
      }
      onStaged(out, names);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const browse = async () => {
    if (disabled || busy) return;
    if (!isTauri()) {
      setErr("Open the desktop app to add files.");
      return;
    }
    try {
      const sel = await open({
        multiple,
        directory: false,
        filters: [{ name: "Media", extensions }],
      });
      if (!sel) return;
      await handlePaths(Array.isArray(sel) ? sel : [sel]);
    } catch (e) {
      setErr(String(e));
    }
  };

  useEffect(() => {
    if (!isTauri() || disabled) return;
    let un: (() => void) | undefined;
    let cancelled = false;
    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (disabled) return;
          const p = event.payload;
          if (p.type === "over" || p.type === "enter") setDrag(true);
          else if (p.type === "leave") setDrag(false);
          else if (p.type === "drop") {
            setDrag(false);
            if (p.paths && p.paths.length) handlePaths(p.paths);
          }
        })
        .then((f) => { if (cancelled) f(); else un = f; })
        .catch(() => {});
    } catch {
      /* webview APIs only exist inside the desktop shell */
    }
    return () => { cancelled = true; if (un) un(); };
  }, [disabled, extensions.join(","), multiple]); // eslint-disable-line

  return (
    <div>
      <div
        className={`upload${drag ? " drag" : ""}`}
        onClick={browse}
        role="button"
        aria-disabled={disabled}
      >
        <div className="upload-label">{busy ? "Uploading..." : label}</div>
        <div className="upload-sub">
          {subLabel} · <span className="browse">{multiple ? "Browse files" : "Browse file"}</span>
        </div>
      </div>
      {err && <div className="error" style={{ marginTop: "0.5rem", fontSize: "0.8rem" }}>{err}</div>}
    </div>
  );
}
