// Cg — sidebar: brand, Video Editing group, Spoofer group, Settings, footer.
import { useEffect, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { Sf, xf } from "../lib/constants";
import type { Theme } from "../lib/theme";
import {
  BrandLogo, IconVideo, IconWand, IconType, IconGhost, IconImage, IconConvert,
  IconSettings, IconSun, IconMoon, IconChevron,
} from "./icons";

interface SubItem { to: string; label: string; icon?: React.ReactNode; dot?: string }

function NavGroup(props: {
  label: string; icon: React.ReactNode; items: SubItem[]; activePrefix: string; pathname: string;
}) {
  const { label, icon, items, activePrefix, pathname } = props;
  const [open, setOpen] = useState(pathname.startsWith(activePrefix));
  useEffect(() => { if (pathname.startsWith(activePrefix)) setOpen(true); }, [pathname, activePrefix]);
  return (
    <div className="nav-group">
      <button className="nav-group-header" onClick={() => setOpen((o) => !o)}>
        <span className="nav-group-icon">{icon}</span>
        <span className="nav-label">{label}</span>
        <span className="nav-chevron" style={{ transform: open ? "rotate(180deg)" : "none" }}>
          <IconChevron size={14} />
        </span>
      </button>
      {open && (
        <div className="nav-items">
          {items.map((it) => (
            <Link
              key={it.to}
              to={it.to}
              className={`nav-sub-item${pathname === it.to ? " active" : ""}`}
            >
              {it.dot ? (
                <span className="nav-platform-dot" style={{ color: it.dot, background: it.dot }} />
              ) : (
                <span className="nav-platform-icon">{it.icon}</span>
              )}
              <span>{it.label}</span>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}

export function Sidebar(props: { theme: Theme; setTheme: (t: Theme) => void }) {
  const { theme, setTheme } = props;
  const { pathname } = useLocation();

  const editorItems: SubItem[] = [
    { to: "/editor/subtitles", label: "Auto subtitles", icon: <IconWand size={16} /> },
    { to: "/editor/snapchat", label: "Snapchat text", icon: <IconGhost size={16} /> },
    { to: "/editor/image-converter", label: "Image converter", icon: <IconImage size={16} /> },
    { to: "/editor/video-converter", label: "Video converter", icon: <IconConvert size={16} /> },
  ];
  const spooferItems: SubItem[] = Sf.map((p) => ({
    to: `/spoofer/${p.id}`, label: p.label, dot: p.color,
  }));

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <div className="brand-mark" aria-hidden>
          <BrandLogo size={40} />
        </div>
        <div>
          <div className="brand-name">{xf}</div>
          <div className="brand-tag">Studio</div>
        </div>
      </div>

      <NavGroup label="Video Editing" icon={<IconVideo size={18} />} items={editorItems} activePrefix="/editor" pathname={pathname} />
      <NavGroup label="Spoofer" icon={<IconType size={18} />} items={spooferItems} activePrefix="/spoofer" pathname={pathname} />

      <Link to="/settings" className={`nav-standalone${pathname === "/settings" ? " active" : ""}`} style={{ marginTop: "0.5rem" }}>
        <span className="nav-platform-icon"><IconSettings size={16} /></span>
        <span>Settings</span>
      </Link>

      <div className="sidebar-footer">
        <div className="theme-toggle" role="group" aria-label="Color theme">
          <button aria-pressed={theme === "light"} onClick={() => setTheme("light")}>
            <IconSun size={15} /> Light
          </button>
          <button aria-pressed={theme === "dark"} onClick={() => setTheme("dark")}>
            <IconMoon size={15} /> Dark
          </button>
        </div>
      </div>
    </aside>
  );
}
