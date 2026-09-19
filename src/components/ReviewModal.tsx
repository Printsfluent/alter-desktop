// Lf — review-outputs swipe deck. RIGHT = keep, LEFT = reject; finishing deletes
// the rejected files. Constants: commit px = 110 (Xi), anim ms = 260 (oa).
import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "../lib/tauri";
import { REVIEW_COMMIT_PX as XI, REVIEW_MS as OA } from "../lib/constants";
import { extname } from "../lib/helpers";
import type { OutputRecord } from "../lib/types";

const isVideo = (name: string) => ["mp4", "mov", "mkv", "webm", "avi", "gif"].includes(extname(name));

export function ReviewModal(props: {
  items: OutputRecord[];
  onClose: () => void;
  openItem: (o: OutputRecord) => void;
  onDeleted: () => void;
  deleteItem: (id: string) => Promise<void>;
  markReviewed: (ids: string[]) => Promise<void>;
}) {
  const { items, onClose, openItem, onDeleted, deleteItem, markReviewed } = props;
  const [pos, setPos] = useState(0);
  const [decisions, setDecisions] = useState<Record<string, "keep" | "reject">>({});
  const [history, setHistory] = useState<number[]>([]);
  const [dragX, setDragX] = useState(0);
  const [leaving, setLeaving] = useState<null | "keep" | "reject">(null);
  const dragging = useRef(false);
  const startX = useRef(0);

  const total = items.length;
  const done = pos >= total;
  const rejects = Object.values(decisions).filter((d) => d === "reject").length;
  const keeps = Object.values(decisions).filter((d) => d === "keep").length;

  const commit = (dir: "keep" | "reject") => {
    if (done || leaving) return;
    const item = items[pos];
    setLeaving(dir);
    setDragX(dir === "keep" ? 1 : -1);
    if (dir === "keep") markReviewed([item.id]).catch(() => {});
    window.setTimeout(() => {
      setDecisions((d) => ({ ...d, [item.id]: dir }));
      setHistory((h) => [...h, pos]);
      setPos((p) => p + 1);
      setLeaving(null);
      setDragX(0);
    }, OA);
  };

  const undo = () => {
    if (!history.length) return;
    const prev = history[history.length - 1];
    setHistory((h) => h.slice(0, -1));
    setDecisions((d) => {
      const copy = { ...d };
      delete copy[items[prev].id];
      return copy;
    });
    setPos(prev);
  };

  const finish = async () => {
    for (const [id, d] of Object.entries(decisions)) if (d === "reject") await deleteItem(id).catch(() => {});
    onDeleted();
    onClose();
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") finish();
      else if (e.key === "ArrowRight") commit("keep");
      else if (e.key === "ArrowLeft") commit("reject");
      else if (e.key === "Backspace") undo();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }); // re-bind each render so closures are fresh

  const onDown = (e: React.PointerEvent) => {
    if (done || leaving) return;
    dragging.current = true;
    startX.current = e.clientX;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };
  const onMove = (e: React.PointerEvent) => {
    if (!dragging.current) return;
    setDragX(e.clientX - startX.current);
  };
  const onUp = () => {
    if (!dragging.current) return;
    dragging.current = false;
    if (dragX > XI) commit("keep");
    else if (dragX < -XI) commit("reject");
    else setDragX(0);
  };

  const header = done ? "All reviewed" : `${total - pos} of ${total} left`;

  return (
    <div className="review-overlay" role="dialog" aria-modal="true">
      <div className="review-panel">
        <div className="review-header">
          <div>
            <strong>Review outputs</strong>
            <div className="muted" style={{ fontSize: "0.78rem" }}>{header}</div>
          </div>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            {!done && <button className="btn ghost small" onClick={async () => { await markReviewed(items.slice(pos).map((i) => i.id)); onClose(); }}>Approve all</button>}
            {!done && <button className="btn ghost small" onClick={() => { const d = { ...decisions }; items.slice(pos).forEach((i) => (d[i.id] = "reject")); setDecisions(d); setPos(total); }}>Remove all</button>}
            <button className="icon-btn" onClick={finish} aria-label="close">✕</button>
          </div>
        </div>

        <div className="progress-track" style={{ marginBottom: "1rem" }}>
          <div className="progress-bar" style={{ width: `${total ? (pos / total) * 100 : 100}%` }} />
        </div>

        {!done ? (
          <>
            <div className="review-deck">
              {items.slice(pos, pos + 3).map((item, idx) => {
                const top = idx === 0;
                const tintOpacity = top ? Math.min(Math.abs(dragX) / XI, 1) * 0.55 : 0;
                const transform = top
                  ? leaving
                    ? `translateX(${leaving === "keep" ? 160 : -160}%) rotate(${leaving === "keep" ? 22 : -22}deg)`
                    : `translateX(${dragX}px) rotate(${dragX * 0.05}deg)`
                  : `translateY(${idx * 10}px) scale(${1 - idx * 0.04})`;
                return (
                  <div
                    key={item.id}
                    className="review-card"
                    style={{
                      transform,
                      zIndex: 10 - idx,
                      transition: top && !dragging.current ? `transform ${OA}ms ease, opacity ${OA}ms ease` : "none",
                    }}
                    onPointerDown={top ? onDown : undefined}
                    onPointerMove={top ? onMove : undefined}
                    onPointerUp={top ? onUp : undefined}
                  >
                    {isVideo(item.file_name) ? (
                      <video src={convertFileSrc(item.file_path)} autoPlay muted loop playsInline />
                    ) : (
                      <img src={convertFileSrc(item.file_path)} alt={item.file_name} />
                    )}
                    {top && dragX > 12 && <div className="review-tint keep" style={{ opacity: tintOpacity }} />}
                    {top && dragX < -12 && <div className="review-tint nope" style={{ opacity: tintOpacity }} />}
                    {top && dragX > 12 && <div className="review-badge keep" style={{ opacity: tintOpacity / 0.55 }}>KEEP</div>}
                    {top && dragX < -12 && <div className="review-badge nope" style={{ opacity: tintOpacity / 0.55 }}>NOPE</div>}
                    <div className="review-card-name">
                      <div style={{ fontSize: "0.85rem", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{item.file_name}</div>
                      <button className="btn ghost small" style={{ marginTop: "0.4rem" }} onClick={() => openItem(item)}>Open</button>
                    </div>
                  </div>
                );
              })}
            </div>

            <div className="review-actions">
              <button className="review-action reject" onClick={() => commit("reject")} title="Delete">✕</button>
              <button className="review-action" onClick={undo} disabled={!history.length} title="Undo">↶</button>
              <button className="review-action keep" onClick={() => commit("keep")} title="Keep">✓</button>
            </div>
            <div className="muted" style={{ textAlign: "center", fontSize: "0.78rem", marginTop: "0.6rem" }}>
              ✓ {keeps} keep · ✕ {rejects} delete
            </div>
            <div className="muted" style={{ textAlign: "center", fontSize: "0.72rem", marginTop: "0.3rem" }}>
              Swipe the card, or use the buttons · ← / → · Backspace = undo
            </div>
          </>
        ) : (
          <div style={{ textAlign: "center", padding: "2rem 0" }}>
            {rejects > 0 ? (
              <>
                <p className="warning">Deleting removes the {rejects} rejected file(s) from disk. This can't be undone.</p>
                <div style={{ display: "flex", gap: "0.75rem", justifyContent: "center", marginTop: "1rem" }}>
                  <button className="btn ghost" onClick={onClose}>Keep all</button>
                  <button className="btn danger" onClick={finish}>Delete {rejects}</button>
                </div>
              </>
            ) : (
              <button className="btn primary" onClick={onClose}>Done</button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
