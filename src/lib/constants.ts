// All literal registries / default config objects from the spec (§3, §4, §6, §9).
import type {
  PlatformId, ExifConfig, SpoofConfig, ImageSpoofConfig, RedditSpoofConfig,
  SubtitleConfig,
} from "./types";

// accent colors (la)
export const la = { reddit: "#ff4500", twitter: "#1d9bf0", instagram: "#e1306c" } as const;

// platform registry (Sf)
export const Sf: { id: PlatformId; label: string; color: string }[] = [
  { id: "reddit", label: "Reddit", color: la.reddit },
  { id: "twitter", label: "Twitter / X", color: la.twitter },
  { id: "instagram", label: "Instagram / Threads", color: la.instagram },
];

// gate membership lists
export const Vv: PlatformId[] = ["instagram", "reddit", "twitter"];
export const Hv: PlatformId[] = ["reddit", "twitter"];

// accepted extensions
export const rr = ["mp4", "mov", "mkv", "webm", "avi"]; // video inputs
export const Ll = ["jpg", "jpeg", "png", "webp", "heic", "heif"]; // image inputs + aa() classifier
export const Xv = [...Ll, ...rr, "gif"]; // reddit/twitter combined

// behavior constants
export const Qv = 8; // max parallel spoofs
export const maxCopies = (image: boolean, c: boolean) => (c || image ? 100 : 50); // $r

// app identity
export const xf = "Alter";
export const Nf = "alter-theme"; // theme localStorage key
export const ag = "dark"; // default theme

// EXIF defaults (wf)
export const wf: ExifConfig = { mode: "random", pool: "all", device_id: null };

// video spoof config default ($v)
export const $v: SpoofConfig = {
  strength: "medium",
  outputs_per_source: 3,
  video_spoof: true,
  speed_spoof: true,
  bw_pct: 0,
  mirror_pct: 0,
  trim_secs: 0,
  trim_start: true,
  trim_end: true,
};

// image spoof config default (Wv)
export const Wv: ImageSpoofConfig = { copies: 5, exif: { ...wf } };

// reddit/twitter config default (Bv)
export const Bv: RedditSpoofConfig = {
  copies: 5,
  strength: "medium",
  mirror: false,
  trim_secs: 0,
  trim_start: true,
  trim_end: true,
  exif: { ...wf },
};

// strength presets (Pd)
export const Pd = [
  { id: "minimal", label: "Minimal", hint: "Barely visible — subtle color + zoom + metadata" },
  { id: "medium", label: "Medium", hint: "Balanced — color, zoom, trim, light noise, sine drift" },
  { id: "hard", label: "Hard", hint: "Strong — bigger shifts, border, noise, audio pitch" },
  { id: "extreme", label: "Extreme", hint: "Maximum — aggressive on every axis" },
] as const;

// EXIF camera buttons (write these exact shapes)
export const exifButtons: {
  key: string; label: string; value: ExifConfig; title: string;
}[] = [
  { key: "random", label: "Random", value: { mode: "random", pool: "all", device_id: null }, title: "A random device per photo, freshly drawn on every run" },
  { key: "ios", label: "iOS", value: { mode: "random", pool: "ios", device_id: null }, title: "An iPhone — pick a specific model below, or random" },
  { key: "android", label: "Android", value: { mode: "random", pool: "android", device_id: null }, title: "An Android phone — pick a specific model below, or random" },
  { key: "off", label: "Off", value: { mode: "off", pool: "all", device_id: null }, title: "Write no EXIF at all" },
];

// ---------- editor / caption constants ----------

// export template (ds)
export const ds = {
  trim_start: null, trim_end: null, export_width: 1080, export_height: 1920,
  export_fps: 30, subtitle_enabled: true, caption_text: null,
  caption_style: { position: "bottom", font_size: 48, color: "#ffffff" }, // Zv
  effect: "none", fade_in: false, fade_out: false, fade_duration: 0.5,
  audio_path: null, audio_volume: 0.85, subtitles_path: null,
  text_clip_start: null, text_clip_end: null, overlay_preset: "subtitles", overlay_y: 0.5,
};
export const Zv = { position: "bottom", font_size: 48, color: "#ffffff" };
export const qv = { position: "bottom", font_size: 44, color: "#ffffff" };
export const eg = { position: "center", font_size: 52, color: "#ffffff" };

// default subtitle-burn config (ng)
export const ng: SubtitleConfig = {
  mode: "highlight", font: "Arial Black", font_size: 64, outline: 4,
  primary_color: "#ffffff", highlight_color: "#ffe000",
  use_box: false, position: 0.78, max_words: 4, uppercase: false,
};

// 12 subtitle presets (rg)
export const rg: { id: string; label: string; config: SubtitleConfig }[] = [
  { id: "classic", label: "Classic", config: { mode: "highlight", font: "Arial Black", font_size: 64, outline: 4, primary_color: "#ffffff", highlight_color: "#ffe000", use_box: false, position: 0.78, max_words: 4, uppercase: false } },
  { id: "beasty", label: "Beasty", config: { mode: "highlight", font: "Impact", font_size: 72, outline: 5, primary_color: "#ffffff", highlight_color: "#39ff14", use_box: false, position: 0.78, max_words: 3, uppercase: true } },
  { id: "bold-yellow", label: "Bold Yellow", config: { mode: "word", font: "Impact", font_size: 78, outline: 6, primary_color: "#ffe000", highlight_color: "#ffe000", use_box: false, position: 0.78, max_words: 1, uppercase: true } },
  { id: "clean", label: "Clean", config: { mode: "highlight", font: "Helvetica", font_size: 56, outline: 3, primary_color: "#ffffff", highlight_color: "#3ea6ff", use_box: false, position: 0.78, max_words: 5, uppercase: false } },
  { id: "boxed", label: "Boxed", config: { mode: "word", font: "Arial Black", font_size: 60, outline: 0, primary_color: "#ffffff", highlight_color: "#ffffff", use_box: true, position: 0.78, max_words: 1, uppercase: true } },
  { id: "pop-pink", label: "Pop Pink", config: { mode: "highlight", font: "Futura", font_size: 66, outline: 4, primary_color: "#ffffff", highlight_color: "#ff4fa3", use_box: false, position: 0.78, max_words: 3, uppercase: true } },
  { id: "red-alert", label: "Red Alert", config: { mode: "highlight", font: "Impact", font_size: 70, outline: 5, primary_color: "#ffffff", highlight_color: "#ff3b30", use_box: false, position: 0.78, max_words: 3, uppercase: true } },
  { id: "ice-blue", label: "Ice Blue", config: { mode: "word", font: "Impact", font_size: 76, outline: 5, primary_color: "#00d4ff", highlight_color: "#00d4ff", use_box: false, position: 0.78, max_words: 1, uppercase: false } },
  { id: "gold", label: "Gold", config: { mode: "highlight", font: "Gill Sans", font_size: 60, outline: 4, primary_color: "#ffffff", highlight_color: "#ffd700", use_box: false, position: 0.78, max_words: 4, uppercase: false } },
  { id: "purple", label: "Purple", config: { mode: "highlight", font: "Arial Black", font_size: 64, outline: 4, primary_color: "#ffffff", highlight_color: "#a855f7", use_box: false, position: 0.78, max_words: 3, uppercase: true } },
  { id: "minimal", label: "Minimal", config: { mode: "word", font: "Helvetica", font_size: 50, outline: 2, primary_color: "#ffffff", highlight_color: "#ffffff", use_box: false, position: 0.78, max_words: 1, uppercase: false } },
  { id: "subtle-soft", label: "Subtle", config: { mode: "highlight", font: "Verdana", font_size: 52, outline: 3, primary_color: "#f5f5f5", highlight_color: "#ffb347", use_box: false, position: 0.78, max_words: 5, uppercase: false } },
];

// font CSS map (tg)
export const tg: { id: string; label: string; css: string }[] = [
  { id: "Arial Black", label: "Arial Black", css: "'Arial Black', sans-serif" },
  { id: "Impact", label: "Impact", css: "Impact, sans-serif" },
  { id: "Helvetica", label: "Helvetica", css: "Helvetica, Arial, sans-serif" },
  { id: "Futura", label: "Futura", css: "Futura, sans-serif" },
  { id: "Gill Sans", label: "Gill Sans", css: "'Gill Sans', sans-serif" },
  { id: "Verdana", label: "Verdana", css: "Verdana, sans-serif" },
];
export const bf = (font: string) => tg.find((f) => f.id === font)?.css || "sans-serif";

// 17-language list (lg)
export const lg: { code: string; label: string }[] = [
  { code: "auto", label: "Auto-detect" },
  { code: "nl", label: "Nederlands" },
  { code: "en", label: "English" },
  { code: "es", label: "Español" },
  { code: "fr", label: "Français" },
  { code: "de", label: "Deutsch" },
  { code: "pt", label: "Português" },
  { code: "it", label: "Italiano" },
  { code: "pl", label: "Polski" },
  { code: "tr", label: "Türkçe" },
  { code: "ru", label: "Русский" },
  { code: "ar", label: "العربية" },
  { code: "hi", label: "हिन्दी" },
  { code: "id", label: "Indonesia" },
  { code: "ja", label: "日本語" },
  { code: "ko", label: "한국어" },
  { code: "zh", label: "中文" },
];

// export-type tag map (Id)
export const Id: Record<string, { label: string; className: string }> = {
  snapchat: { label: "Snapchat", className: "type-snapchat" },
  subtitles: { label: "Subtitles", className: "type-subtitles" },
  spoof: { label: "Spoof", className: "type-spoof" },
};

// page sizes
export const PAGE_SIZE = 10; // Zi / An

// converter lists
export const Gv = ["jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "tif", "bmp", "gif"];
export const Kv = ["jpg", "png", "webp", "heic", "tiff", "bmp"];
export const Jv = ["mp4", "mov", "mkv", "webm", "avi", "gif", "m4v"];
export const Yv = ["mp4", "mov", "mkv", "webm", "avi", "gif"];

// review modal animation constants
export const REVIEW_COMMIT_PX = 110; // Xi
export const REVIEW_MS = 260; // oa
