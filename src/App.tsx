import { useState } from "react";
import { BrowserRouter, Routes, Route, Navigate, useParams } from "react-router-dom";
import { Sidebar } from "./components/Sidebar";
import { applyTheme, readTheme, type Theme } from "./lib/theme";
import { Spoofer } from "./pages/Spoofer";
import { Editor } from "./pages/Editor";
import { Converter } from "./pages/Converter";
import { Settings } from "./pages/Settings";
import type { PlatformId } from "./lib/types";

// /platform/:id → /spoofer/:id  (Kg)
function PlatformRedirect() {
  const { id } = useParams();
  return <Navigate to={`/spoofer/${id ?? "reddit"}`} replace />;
}

export function App() {
  const [theme, setThemeState] = useState<Theme>(readTheme());
  const setTheme = (t: Theme) => { applyTheme(t); setThemeState(t); };

  return (
    <BrowserRouter>
      <div className="app-shell">
        <Sidebar theme={theme} setTheme={setTheme} />
        <main className="content">
          <Routes>
            <Route path="/" element={<Navigate to="/editor/subtitles" replace />} />

            {/* editor */}
            <Route path="/editor/subtitles" element={<Editor mode="captions" />} />
            <Route path="/editor/snapchat" element={<Editor mode="snapchat" />} />
            <Route path="/editor/image-converter" element={<Converter kind="image" />} />
            <Route path="/editor/video-converter" element={<Converter kind="video" />} />
            <Route path="/editor" element={<Navigate to="/editor/subtitles" replace />} />
            <Route path="/editor/video" element={<Navigate to="/editor/subtitles" replace />} />
            <Route path="/editor/outputs" element={<Navigate to="/editor/subtitles" replace />} />
            <Route path="/editor/quick" element={<Navigate to="/editor/subtitles" replace />} />
            <Route path="/editor/templates" element={<Navigate to="/editor/subtitles" replace />} />
            <Route path="/editor/batch" element={<Navigate to="/editor/subtitles" replace />} />

            {/* spoofer */}
            <Route path="/spoofer/reddit" element={<Spoofer platformId={"reddit" as PlatformId} />} />
            <Route path="/spoofer/twitter" element={<Spoofer platformId={"twitter" as PlatformId} />} />
            <Route path="/spoofer/instagram" element={<Spoofer platformId={"instagram" as PlatformId} />} />
            <Route path="/spoofer/threads" element={<Navigate to="/spoofer/instagram" replace />} />
            <Route path="/spoofer/outputs" element={<Navigate to="/spoofer/reddit" replace />} />
            <Route path="/outputs" element={<Navigate to="/spoofer/reddit" replace />} />

            <Route path="/settings" element={<Settings />} />
            <Route path="/platform/:id" element={<PlatformRedirect />} />

            <Route path="*" element={<Navigate to="/editor/subtitles" replace />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
