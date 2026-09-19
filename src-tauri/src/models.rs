// Serde structs mirroring the frontend payloads and DB records.
use serde::{Deserialize, Serialize};

fn t() -> bool { true }

// ----- spoof configs -----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifConfig {
    pub mode: String,   // random | device | off
    pub pool: String,   // all | ios | android
    pub device_id: Option<String>,
}
impl Default for ExifConfig {
    fn default() -> Self {
        Self { mode: "random".into(), pool: "all".into(), device_id: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoofConfig {
    pub strength: String,
    pub outputs_per_source: u32,
    #[serde(default = "t")]
    pub video_spoof: bool,
    #[serde(default = "t")]
    pub speed_spoof: bool,
    #[serde(default)]
    pub bw_pct: u32,
    #[serde(default)]
    pub mirror_pct: u32,
    #[serde(default)]
    pub trim_secs: f64,
    #[serde(default = "t")]
    pub trim_start: bool,
    #[serde(default = "t")]
    pub trim_end: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSpoofConfig {
    pub copies: u32,
    #[serde(default)]
    pub exif: ExifConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditSpoofConfig {
    pub copies: u32,
    pub strength: String,
    #[serde(default)]
    pub mirror: bool,
    #[serde(default)]
    pub trim_secs: f64,
    #[serde(default = "t")]
    pub trim_start: bool,
    #[serde(default = "t")]
    pub trim_end: bool,
    #[serde(default)]
    pub exif: ExifConfig,
}

// ----- DB records -----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub platform: String,
    pub input_path: String,
    pub output_path: Option<String>,
    pub status: String, // queued | processing | completed | failed
    pub progress: i64,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputRecord {
    pub id: String,
    pub job_id: String,
    pub platform: String,
    pub file_name: String,
    pub file_path: String,
    pub created_at: String,
    pub reviewed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifDevice {
    pub id: String,
    pub label: String,
    pub os: String, // ios | android
    pub make: String,
    pub model: String,
    pub software: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub valid: bool,
    pub license_type: Option<String>,
    pub expires_at: Option<String>,
    pub device_bound: bool,
    pub team_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPaths {
    pub uploads: String,
    pub outputs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64,
}

// ----- editor / caption -----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleConfig {
    pub mode: String,
    pub font: String,
    pub font_size: f64,
    pub outline: f64,
    pub primary_color: String,
    pub highlight_color: String,
    pub use_box: bool,
    pub position: f64,
    pub max_words: u32,
    pub uppercase: bool,
}
impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            mode: "highlight".into(), font: "Arial Black".into(), font_size: 64.0, outline: 4.0,
            primary_color: "#ffffff".into(), highlight_color: "#ffe000".into(),
            use_box: false, position: 0.78, max_words: 4, uppercase: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionStyle {
    pub position: String,
    pub font_size: f64,
    pub color: String,
}
impl Default for CaptionStyle {
    fn default() -> Self { Self { position: "bottom".into(), font_size: 48.0, color: "#ffffff".into() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionWord { pub word: String, pub start: f64, pub end: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionSegment { pub start: f64, pub end: f64, pub text: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionResult {
    pub srt_path: String,
    pub segments: Vec<CaptionSegment>,
    pub words: Vec<CaptionWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTemplate {
    pub id: String,
    pub name: String,
    pub subtitle_enabled: bool,
    pub caption_text: Option<String>,
    pub subtitle_config: Option<SubtitleConfig>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoJobRecord {
    pub id: String,
    pub mode: String,
    pub template_id: Option<String>,
    pub template_name: Option<String>,
    pub input_path: String,
    pub output_path: Option<String>,
    pub status: String,
    pub progress: i64,
    pub error: Option<String>,
    pub render_options: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoOutputRecord {
    pub id: String,
    pub job_id: String,
    pub template_name: Option<String>,
    pub file_name: String,
    pub file_path: String,
    pub created_at: String,
}
