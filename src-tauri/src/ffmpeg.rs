// ffmpeg / ffprobe location, probing, progress-tracked runs, filter escaping.
use crate::models::ProbeResult;
use crate::storage;
use once_cell::sync::Lazy;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn cmd(bin: &str) -> Command {
    let mut c = Command::new(bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

fn looks_runnable(p: &std::path::Path) -> bool {
    cmd(p.to_str().unwrap_or("")).arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn with_exe(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn resolve_bin(name: &str) -> String {
    let file = with_exe(name);
    let mut candidates: Vec<PathBuf> = Vec::new();
    candidates.push(storage::data_root().join("bin").join(&file));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(&file));
            candidates.push(dir.join("bin").join(&file));
            // bundled next to a macOS .app executable: ../Resources/bin
            if let Some(res) = dir.parent() {
                candidates.push(res.join("Resources").join("bin").join(&file));
            }
        }
    }
    candidates.push(PathBuf::from("/opt/homebrew/bin").join(&file));
    candidates.push(PathBuf::from("/usr/local/bin").join(&file));
    if let Ok(pf) = std::env::var("ProgramFiles") {
        candidates.push(PathBuf::from(pf).join("ffmpeg").join("bin").join(&file));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Microsoft").join("WinGet").join("Links").join(&file));
    }
    candidates.push(PathBuf::from(&file));
    candidates.push(PathBuf::from(name));
    for p in candidates {
        if looks_runnable(&p) {
            return p.to_string_lossy().into_owned();
        }
    }
    file
}

static FFMPEG: Lazy<String> = Lazy::new(|| resolve_bin("ffmpeg"));
static FFPROBE: Lazy<String> = Lazy::new(|| resolve_bin("ffprobe"));

pub fn ffmpeg_bin() -> String { FFMPEG.clone() }
pub fn ffprobe_bin() -> String { FFPROBE.clone() }

pub fn ensure_ffmpeg() -> Result<(), String> {
    if !looks_runnable(std::path::Path::new(&*FFMPEG)) {
        return Err("ffmpeg is not installed. On Mac: brew install ffmpeg. On Windows: winget install Gyan.FFmpeg — then restart Alter.".into());
    }
    if !looks_runnable(std::path::Path::new(&*FFPROBE)) {
        return Err("ffprobe is not installed. Install ffmpeg (it includes ffprobe) and restart Alter.".into());
    }
    Ok(())
}

fn ffprobe_json(args: &[&str]) -> Option<serde_json::Value> {
    let out = cmd(&ffprobe_bin()).args(args).output().ok()?;
    serde_json::from_slice(&out.stdout).ok()
}

// probe_video — dimensions + duration (spec §9: two queries, -of json)
pub fn probe_video(path: &str) -> Result<ProbeResult, String> {
    if !std::path::Path::new(path).exists() {
        return Err("Input video not found".into());
    }
    let j = ffprobe_json(&["-v", "error", "-show_entries", "format=duration",
        "-show_entries", "stream=codec_type,sample_rate", "-of", "json", path])
        .ok_or_else(|| "ffprobe failed: ".to_string())?;
    let duration_secs: f64 = j["format"]["duration"].as_str().and_then(|s| s.parse().ok())
        .or_else(|| j["format"]["duration"].as_f64())
        .unwrap_or(0.0);

    let j2 = ffprobe_json(&["-v", "error", "-select_streams", "v:0",
        "-show_entries", "stream=width,height", "-of", "json", path])
        .ok_or_else(|| "ffprobe failed: ".to_string())?;
    let s = &j2["streams"][0];
    let width = s["width"].as_u64().unwrap_or(0) as u32;
    let height = s["height"].as_u64().unwrap_or(0) as u32;

    Ok(ProbeResult { width, height, duration_secs })
}

pub fn has_audio(path: &str) -> bool {
    ffprobe_json(&["-v", "error", "-show_entries", "stream=codec_type", "-of", "json", path])
        .and_then(|j| j["streams"].as_array().map(|streams| {
            streams.iter().any(|s| s["codec_type"].as_str() == Some("audio"))
        }))
        .unwrap_or(false)
}

// audio sample rate of the first audio stream (for asetrate pitch shifting)
pub fn audio_sample_rate(path: &str) -> Option<u32> {
    let j = ffprobe_json(&["-v", "error", "-select_streams", "a:0",
        "-show_entries", "stream=sample_rate", "-of", "json", path])?;
    j["streams"][0]["sample_rate"].as_str().and_then(|s| s.parse().ok())
}

// Run ffmpeg with -progress on stdout; report 0..100 through `on_progress`.
// `cancel` (when set) kills the child and returns "Cancelled by user".
pub fn run_with_progress(
    args: &[String],
    total_secs: f64,
    cancel: &Arc<AtomicBool>,
    mut on_progress: impl FnMut(i64),
) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Cancelled by user".into());
    }
    let mut full: Vec<String> = vec!["-y".into(), "-hide_banner".into(), "-loglevel".into(), "error".into(),
        "-progress".into(), "pipe:1".into(), "-nostats".into()];
    full.extend(args.iter().cloned());

    let mut child = cmd(&ffmpeg_bin())
        .args(&full)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg spawn failed: {e}"))?;

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Cancelled by user".into());
        }
        let line = match line { Ok(l) => l, Err(_) => break };
        if let Some(v) = line.strip_prefix("out_time_ms=") {
            if let Ok(ms) = v.trim().parse::<i64>() {
                if total_secs > 0.0 {
                    let pct = ((ms as f64 / 1_000_000.0) / total_secs * 100.0).clamp(0.0, 99.0);
                    on_progress(pct as i64);
                }
            }
        } else if line.starts_with("progress=end") {
            on_progress(100);
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        // capture stderr for the error
        let mut err = String::new();
        if let Some(mut se) = child.stderr.take() {
            use std::io::Read;
            let _ = se.read_to_string(&mut err);
        }
        return Err(format!("ffmpeg failed: {}", err.lines().last().unwrap_or("")));
    }
    Ok(())
}

// Simple blocking run (no progress) used by converters / heic transcode.
pub fn run(args: &[String]) -> Result<(), String> {
    let out = cmd(&ffmpeg_bin()).arg("-y").args(args).output().map_err(|e| format!("ffmpeg spawn failed: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("ffmpeg failed: {}", err.lines().last().unwrap_or("ffmpeg produced no output")));
    }
    Ok(())
}

// escape_filter_path — backslash-prefix ' , : ; [ \ ]
pub fn escape_filter_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    for ch in p.chars() {
        if matches!(ch, '\'' | ',' | ':' | ';' | '[' | '\\' | ']') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}
