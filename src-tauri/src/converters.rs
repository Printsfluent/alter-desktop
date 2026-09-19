// convert_image_file / convert_video_file (spec §9 converter section).
use crate::ffmpeg;
use crate::storage::outputs_dir;
use image::ImageFormat;
use std::path::{Path, PathBuf};

fn resolve_out_dir(output_dir: Option<&str>) -> PathBuf {
    let d = match output_dir {
        Some(p) => PathBuf::from(p),
        None => outputs_dir().join("Converted"),
    };
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn convert_image(input: &str, target: &str, quality: Option<u8>, base_name: &str, output_dir: Option<&str>) -> Result<String, String> {
    // accepted targets (spec validation set); jpeg/tif/heif/gif/avif are rejected
    if !matches!(target, "jpg" | "png" | "bmp" | "webp" | "heic" | "tiff") {
        return Err(format!("Unsupported image target: {target}"));
    }
    let dir = resolve_out_dir(output_dir);
    let out: PathBuf = dir.join(format!("{base_name}.{target}"));
    let out_s = out.to_string_lossy().to_string();

    if target == "heic" {
        return convert_heic(input, &out_s);
    }

    let img = image::open(input).map_err(|e| e.to_string())?;
    match target {
        "jpg" => {
            let q = quality.unwrap_or(95).clamp(1, 100);
            let rgb = img.to_rgb8();
            let file = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            let mut w = std::io::BufWriter::new(file);
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, q);
            enc.encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
        }
        "png" => img.save_with_format(&out, ImageFormat::Png).map_err(|e| e.to_string())?,
        "webp" => img.save_with_format(&out, ImageFormat::WebP).map_err(|e| e.to_string())?,
        "bmp" => img.save_with_format(&out, ImageFormat::Bmp).map_err(|e| e.to_string())?,
        "tiff" => img.save_with_format(&out, ImageFormat::Tiff).map_err(|e| e.to_string())?,
        _ => unreachable!(),
    }
    Ok(out_s)
}

// HEIC encode: platform_image_convert (sips) on macOS; ffmpeg fallback elsewhere.
fn convert_heic(input: &str, out_s: &str) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        match std::process::Command::new("sips")
            .args(["-s", "format", "heic", input, "--out", out_s])
            .status()
        {
            Ok(s) if s.success() => return Ok(out_s.to_string()),
            Ok(_) => {}
            Err(e) => return Err(format!("sips not available: {e}")),
        }
    }
    ffmpeg::run(&["-i".into(), input.into(), out_s.to_string()])
        .map_err(|_| "HEIC output not supported on this platform".to_string())?;
    Ok(out_s.to_string())
}

pub fn convert_video(input: &str, target: &str, base_name: &str, output_dir: Option<&str>) -> Result<String, String> {
    ffmpeg::ensure_ffmpeg()?;
    let dir = resolve_out_dir(output_dir);
    let out: PathBuf = dir.join(format!("{base_name}.{target}"));
    let out_s = out.to_string_lossy().to_string();

    let mut args: Vec<String> = vec!["-i".into(), input.into()];
    let codec: Vec<&str> = match target {
        "mp4" | "mov" | "m4v" => vec!["-c:v", "libx264", "-preset", "fast", "-crf", "20", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"],
        "mkv" => vec!["-c:v", "libx264", "-preset", "fast", "-crf", "20", "-c:a", "aac", "-b:a", "192k"],
        "webm" => vec!["-c:v", "libvpx-vp9", "-b:v", "0", "-crf", "32", "-row-mt", "1", "-c:a", "libopus"],
        "avi" => vec!["-c:v", "mpeg4", "-qscale:v", "4", "-c:a", "libmp3lame", "-q:a", "4"],
        "gif" => vec!["-vf", "fps=12,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"],
        other => return Err(format!("Unsupported video target: {other}")),
    };
    args.extend(codec.into_iter().map(String::from));
    args.push(out_s.clone());
    ffmpeg::run(&args)?;
    Ok(out_s)
}

pub fn is_image_ext(ext: &str) -> bool {
    matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp" | "heic" | "heif" | "tiff" | "tif" | "bmp" | "gif")
}

pub fn ext_of(p: &str) -> String {
    Path::new(p).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}
