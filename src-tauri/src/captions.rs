// generate_auto_captions — Whisper transcription (spec §9 captions section).
// Backend order: openai-whisper (python, model "tiny") → bundled whisper-cli +
// ggml-small.bin → whisper-cli + ggml-tiny fallback → graceful error.
use crate::ffmpeg;
use crate::models::{CaptionResult, CaptionSegment, CaptionWord};
use crate::storage::captions_work_dir;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
fn cmd(bin: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut c = Command::new(bin);
    c.creation_flags(0x0800_0000);
    c
}
#[cfg(not(windows))]
fn cmd(bin: &str) -> Command { Command::new(bin) }

fn have(bin: &str) -> bool {
    cmd(bin).arg("--help").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok()
}

// ggml-tiny fallback paths (spec §9 — first existing wins)
fn find_cli_model() -> Option<String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let m = dir.join("resources").join("ggml-small.bin");
            if m.exists() {
                return Some(m.to_string_lossy().to_string());
            }
        }
    }
    for p in [
        "/opt/homebrew/share/whisper-cpp/for-tests-ggml-tiny.bin",
        "/usr/local/share/whisper-cpp/for-tests-ggml-tiny.bin",
        "/opt/homebrew/share/whisper-cpp/ggml-tiny.bin",
        "/usr/local/share/whisper-cpp/ggml-tiny.bin",
    ] {
        if Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    None
}

// caption-audio.wav ← -vn -acodec pcm_s16le -ar 16000 -ac 1
fn extract_audio(video: &str, wav: &PathBuf) -> Result<(), String> {
    ffmpeg::run(&[
        "-i".into(), video.into(), "-vn".into(), "-acodec".into(), "pcm_s16le".into(),
        "-ar".into(), "16000".into(), "-ac".into(), "1".into(), wav.to_string_lossy().to_string(),
    ]).map_err(|e| format!("Could not extract audio: {e}"))
}

fn parse_srt(text: &str) -> Vec<CaptionSegment> {
    let mut segs = Vec::new();
    let parse_ts = |s: &str| -> f64 {
        let s = s.trim().replace(',', ".");
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 3 {
            let h: f64 = parts[0].parse().unwrap_or(0.0);
            let m: f64 = parts[1].parse().unwrap_or(0.0);
            let sec: f64 = parts[2].parse().unwrap_or(0.0);
            h * 3600.0 + m * 60.0 + sec
        } else { 0.0 }
    };
    for block in text.split("\n\n") {
        let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() < 2 { continue; }
        let idx = match lines.iter().position(|l| l.contains("-->")) { Some(i) => i, None => continue };
        if let Some((a, b)) = lines[idx].split_once("-->") {
            let text: String = lines[idx + 1..].join(" ").trim().to_string();
            if !text.is_empty() {
                segs.push(CaptionSegment { start: parse_ts(a), end: parse_ts(b), text });
            }
        }
    }
    segs
}

// caption-audio.json ← -ojf / openai-whisper --output_format all (segments[].words[])
fn parse_words_json(path: &Path) -> Vec<CaptionWord> {
    let mut out = Vec::new();
    let txt = match std::fs::read_to_string(path) { Ok(t) => t, Err(_) => return out };
    let j: serde_json::Value = match serde_json::from_str(&txt) { Ok(v) => v, Err(_) => return out };
    if let Some(segs) = j["segments"].as_array() {
        for seg in segs {
            if let Some(ws) = seg["words"].as_array() {
                for w in ws {
                    let word = w["word"].as_str().or_else(|| w["text"].as_str()).unwrap_or("").trim().to_string();
                    if word.is_empty() { continue; }
                    let start = w["start"].as_f64().unwrap_or(0.0);
                    let end = w["end"].as_f64().unwrap_or(start);
                    out.push(CaptionWord { word, start, end });
                }
            }
        }
    }
    out
}

// bundled whisper-cli sidecar (next to the exe) preferred, else PATH
fn whisper_cli_bin() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in ["whisper-cli", "whisper-cli.exe"] {
                let p = dir.join(name);
                if p.exists() {
                    return p.to_string_lossy().to_string();
                }
            }
        }
    }
    "whisper-cli".to_string()
}

pub fn generate(video: &str, language: Option<String>, emit: &dyn Fn(&str)) -> Result<CaptionResult, String> {
    let dir = captions_work_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let wav = dir.join("caption-audio.wav");
    let out_base = dir.join("caption-audio");

    let cli_bin = whisper_cli_bin();
    let use_python = have("whisper");
    let cli_model = find_cli_model();
    let use_cli = have(&cli_bin) && cli_model.is_some();
    if !use_python && !use_cli {
        return Err("Whisper not found. Install: pip install openai-whisper  OR  brew install whisper-cpp".into());
    }

    emit("Extracting audio...");
    extract_audio(video, &wav)?;
    let lang = language.filter(|l| l != "auto");

    emit("Transcribing — first run can take 1–2 min...");
    if use_python {
        // openai-whisper: <wav> --model tiny --output_format all --word_timestamps True --output_dir <dir> --verbose False [--language <l>]
        let mut c = cmd("whisper");
        c.arg(wav.to_string_lossy().to_string())
            .args(["--model", "tiny", "--output_format", "all", "--word_timestamps", "True",
                   "--output_dir", &dir.to_string_lossy(), "--verbose", "False"]);
        if let Some(l) = &lang { c.args(["--language", l]); }
        c.output().map_err(|e| e.to_string())?;
    } else {
        // whisper-cli: -m <model.bin> -osrt -ojf -of <workdir>/caption-audio [-l <l>] <wav>
        let model = cli_model.unwrap();
        let mut c = cmd(&cli_bin);
        c.args(["-m", &model, "-osrt", "-ojf", "-of", &out_base.to_string_lossy()]);
        if let Some(l) = &lang { c.args(["-l", l]); }
        c.arg(wav.to_string_lossy().to_string());
        c.output().map_err(|e| e.to_string())?;
    }

    let srt_path = dir.join("caption-audio.srt");
    let srt = std::fs::read_to_string(&srt_path).map_err(|_| "Whisper did not produce SRT output".to_string())?;
    let segments = parse_srt(&srt);
    if segments.is_empty() {
        return Err("No speech detected in video".into());
    }
    let words = parse_words_json(&dir.join("caption-audio.json"));
    emit("Captions ready");

    Ok(CaptionResult { srt_path: srt_path.to_string_lossy().to_string(), segments, words })
}
