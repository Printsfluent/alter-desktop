// Shared types mirroring the Rust command payloads / DB records.

export type PlatformId = "reddit" | "twitter" | "instagram" | "threads";

export interface ExifConfig {
  mode: "random" | "device" | "off";
  pool: "all" | "ios" | "android";
  device_id: string | null;
}

// Video spoof config ($v) — sent for Instagram VIDEO
export interface SpoofConfig {
  strength: "minimal" | "medium" | "hard" | "extreme";
  outputs_per_source: number;
  video_spoof: boolean;
  speed_spoof: boolean;
  bw_pct: number;
  mirror_pct: number;
  trim_secs: number;
  trim_start: boolean;
  trim_end: boolean;
}

// Image spoof config (Wv) — sent for Instagram IMAGE
export interface ImageSpoofConfig {
  copies: number;
  exif: ExifConfig;
}

// Reddit/Twitter config (Bv)
export interface RedditSpoofConfig {
  copies: number;
  strength: SpoofConfig["strength"];
  mirror: boolean;
  trim_secs: number;
  trim_start: boolean;
  trim_end: boolean;
  exif: ExifConfig;
}

export interface JobRecord {
  id: string;
  platform: string;
  input_path: string;
  output_path: string | null;
  status: "queued" | "processing" | "completed" | "failed";
  progress: number;
  error: string | null;
  created_at: string;
  updated_at: string;
}

export interface OutputRecord {
  id: string;
  job_id: string;
  platform: string;
  file_name: string;
  file_path: string;
  created_at: string;
  reviewed: number;
}

export interface ExifDevice {
  id: string;
  label: string;
  os: "ios" | "android";
  make: string;
  model: string;
  software: string | null;
}

export interface LicenseStatus {
  valid: boolean;
  license_type: string | null;
  expires_at: string | null;
  device_bound?: boolean;
  team_id: string | null;
  message: string | null;
}

export interface AppPaths {
  uploads: string;
  outputs: string;
}

export interface ProbeResult {
  width: number;
  height: number;
  duration_secs: number;
}

// Subtitle / caption config (SubtitleConfig, 10 fields)
export interface SubtitleConfig {
  mode: "highlight" | "word";
  font: string;
  font_size: number;
  outline: number;
  primary_color: string;
  highlight_color: string;
  use_box: boolean;
  position: number;
  max_words: number;
  uppercase: boolean;
}

export interface VideoTemplate {
  id: string;
  name: string;
  subtitle_enabled: boolean;
  caption_text: string | null;
  subtitle_config: SubtitleConfig | null;
  created_at: string;
  updated_at: string;
}

export interface VideoJobRecord {
  id: string;
  mode: string;
  template_id: string | null;
  template_name: string | null;
  input_path: string;
  output_path: string | null;
  status: string;
  progress: number;
  error: string | null;
  render_options: string | null;
  created_at: string;
  updated_at: string;
}

export interface VideoOutputRecord {
  id: string;
  job_id: string;
  template_name: string | null;
  file_name: string;
  file_path: string;
  created_at: string;
}

export interface CaptionWord {
  word: string;
  start: number;
  end: number;
}
export interface CaptionSegment {
  start: number;
  end: number;
  text: string;
}
export interface CaptionResult {
  srt_path: string;
  segments: CaptionSegment[];
  words: CaptionWord[];
}
