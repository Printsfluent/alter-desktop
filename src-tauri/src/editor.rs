// render_video — video-editor export (subtitle burn / snapchat text) per §9.
use crate::ffmpeg;
use crate::storage::captions_work_dir;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[cfg(windows)]
const WIN_ARIAL_BOLD: &str = "C:/Windows/Fonts/arialbd.ttf";
#[cfg(windows)]
const WIN_ARIAL: &str = "C:/Windows/Fonts/arial.ttf";
#[cfg(not(windows))]
const WIN_ARIAL_BOLD: &str = "/System/Library/Fonts/Supplemental/Arial Bold.ttf";
#[cfg(not(windows))]
const WIN_ARIAL: &str = "/System/Library/Fonts/Supplemental/Arial.ttf";

pub fn render_video(
    input: &str,
    ro: &Value,
    out_dir: &Path,
    base_name: &str,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(i64),
) -> Result<PathBuf, String> {
    ffmpeg::ensure_ffmpeg()?;
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let probe = ffmpeg::probe_video(input).unwrap_or(crate::models::ProbeResult { width: 0, height: 0, duration_secs: 0.0 });
    let has_audio = ffmpeg::has_audio(input);

    let w = ro.get("export_width").and_then(|v| v.as_u64()).unwrap_or(1080);
    let h = ro.get("export_height").and_then(|v| v.as_u64()).unwrap_or(1920);
    let fps = ro.get("export_fps").and_then(|v| v.as_u64()).unwrap_or(30);
    let overlay_preset = ro.get("overlay_preset").and_then(|v| v.as_str()).unwrap_or("subtitles");
    let subtitles_path = ro.get("subtitles_path").and_then(|v| v.as_str());
    let caption_text = ro.get("caption_text").and_then(|v| v.as_str());
    let overlay_y = ro.get("overlay_y").and_then(|v| v.as_f64()).unwrap_or(0.5);

    let mut vf: Vec<String> = Vec::new();
    // letterbox
    vf.push(format!("scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black"));

    if overlay_preset == "snapchat" {
        if let Some(text) = caption_text {
            // fontsize + fontcolor derived from caption_style (eg = 52 / #ffffff)
            let cstyle = ro.get("caption_style");
            let fs = cstyle.and_then(|c| c.get("font_size")).and_then(|v| v.as_f64()).unwrap_or(52.0) as i64;
            let color_hex = cstyle.and_then(|c| c.get("color")).and_then(|v| v.as_str())
                .unwrap_or("#ffffff").trim_start_matches('#').to_string();
            let txt_file = captions_work_dir().join(format!("overlay-{}.txt", uuid::Uuid::new_v4().simple()));
            std::fs::create_dir_all(captions_work_dir()).ok();
            std::fs::write(&txt_file, text).map_err(|e| e.to_string())?;
            let font = ffmpeg::escape_filter_path(WIN_ARIAL_BOLD);
            let tf = ffmpeg::escape_filter_path(&txt_file.to_string_lossy());
            vf.push(format!(
                "drawtext=fontfile='{font}':textfile='{tf}':fontsize={fs}:fontcolor=0x{color_hex}@1.0:x=(w-text_w)/2:y=h*{overlay_y}-(text_h/2):box=1:boxcolor=black@0.55:boxborderw=12:text_align=C"
            ));
        }
    } else if let Some(sp) = subtitles_path {
        let esc = ffmpeg::escape_filter_path(sp);
        vf.push(format!("subtitles='{esc}'"));
    }

    let day = 1 + (probe.duration_secs as i64 % 27);

    let out_path: PathBuf = out_dir.join(format!("{base_name}.mp4"));
    let mut args: Vec<String> = vec!["-i".into(), input.into(), "-vf".into(), vf.join(",")];
    args.extend(["-r".into(), fps.to_string()]);
    args.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "23"].map(String::from));
    if has_audio {
        args.extend(["-c:a", "aac", "-b:a", "192k"].map(String::from));
    } else {
        args.push("-an".into());
    }
    args.extend(["-metadata".into(), format!("creation_time=2026-05-{day:02} 12:00:00")]);
    args.extend(["-movflags", "+faststart"].map(String::from));
    args.push(out_path.to_string_lossy().to_string());

    let _ = WIN_ARIAL; // reserved for regular-weight overlays
    ffmpeg::run_with_progress(&args, probe.duration_secs.max(0.1), cancel, |pct| on_progress(pct))?;
    Ok(out_path)
}
