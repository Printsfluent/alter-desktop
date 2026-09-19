// Minimal inline SVG icon set (stroke = currentColor).
type P = { size?: number };
const s = (size = 18) => ({ width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.8, strokeLinecap: "round" as const, strokeLinejoin: "round" as const });

export const IconVideo = ({ size }: P) => (<svg {...s(size)}><rect x="2" y="5" width="14" height="14" rx="2" /><path d="M16 10l6-3v10l-6-3" /></svg>);
export const IconWand = ({ size }: P) => (<svg {...s(size)}><path d="M15 4V2M15 22v-2M8 9l-6 6 4 4 6-6M13 6l5 5" /><path d="M20 9l1 1" /></svg>);
export const IconType = ({ size }: P) => (<svg {...s(size)}><path d="M4 7V5h16v2M9 20h6M12 5v15" /></svg>);
export const IconGhost = ({ size }: P) => (<svg {...s(size)}><path d="M9 10h.01M15 10h.01M12 2a8 8 0 0 0-8 8v12l3-2 3 2 2-2 2 2 3-2V10a8 8 0 0 0-8-8z" /></svg>);
export const IconImage = ({ size }: P) => (<svg {...s(size)}><rect x="3" y="3" width="18" height="18" rx="2" /><circle cx="9" cy="9" r="1.6" /><path d="M21 15l-5-5L5 21" /></svg>);
export const IconConvert = ({ size }: P) => (<svg {...s(size)}><path d="M4 8h13l-3-3M20 16H7l3 3" /></svg>);
export const IconSettings = ({ size }: P) => (<svg {...s(size)}><circle cx="12" cy="12" r="3" /><path d="M19 12a7 7 0 0 0-.1-1l2-1.5-2-3.4-2.3.9a7 7 0 0 0-1.7-1L14.5 2h-4l-.4 2.5a7 7 0 0 0-1.7 1L6 4.6l-2 3.4L6 9.5a7 7 0 0 0 0 2L4 13l2 3.4 2.3-.9a7 7 0 0 0 1.7 1l.4 2.5h4l.4-2.5a7 7 0 0 0 1.7-1l2.3.9 2-3.4-2-1.5c.06-.33.1-.66.1-1z" /></svg>);
export const IconSun = ({ size }: P) => (<svg {...s(size)}><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.5 1.5M17.5 17.5L19 19M5 19l1.5-1.5M17.5 6.5L19 5" /></svg>);
export const IconMoon = ({ size }: P) => (<svg {...s(size)}><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /></svg>);
export const IconChevron = ({ size }: P) => (<svg {...s(size)} style={{ transition: "transform .15s" }}><path d="M6 9l6 6 6-6" /></svg>);

/** Geometric fold / delta mark — unique to Alter. */
export function BrandLogo({ size = 40 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 40 40" fill="none" aria-hidden>
      <defs>
        <linearGradient id="alter-mark-g" x1="6" y1="4" x2="36" y2="36" gradientUnits="userSpaceOnUse">
          <stop stopColor="#9ff8e4" />
          <stop offset="0.55" stopColor="#2ee6c5" />
          <stop offset="1" stopColor="#0ea88e" />
        </linearGradient>
      </defs>
      <rect width="40" height="40" rx="11" fill="url(#alter-mark-g)" />
      <path d="M10.5 25.2 20 9.5l9.5 15.7H24.4L20 18.2l-4.4 7z" fill="#04221c" />
      <path d="M15.6 27.4h8.8L20 34.2z" fill="#04221c" opacity="0.38" />
    </svg>
  );
}
