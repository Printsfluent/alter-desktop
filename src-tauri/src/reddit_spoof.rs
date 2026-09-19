// Reddit/Twitter unified media uniquifier (spec §9). Auto-routes by extension:
// image → image-crate pipeline with dHash dedup vs the PREVIOUSLY-ACCEPTED output
// (per-strength threshold, up to 14 attempts); video → simplified ffmpeg spoof.
// Mirror is a boolean applied to every output. No fake post header (§9 Correction 1).
use crate::exif;
use crate::ffmpeg;
use crate::models::RedditSpoofConfig;
use crate::prng::Prng;
use crate::storage;
use image::{imageops, DynamicImage, GrayImage, RgbImage};
use std::f64::consts::PI;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct RParams {
    threshold: u32,
    scale1: f64,
    scale2: f64,
    crop_base: f64,
    crop_clamp: f64,
    noise_sigma: f64,
    tier: u8,
    rotation: f64,
    brightness: f64, // video eq=brightness range (d13)
}

fn rparams(strength: &str) -> RParams {
    match strength {
        "minimal" => RParams { threshold: 8, scale1: 0.015, scale2: 0.0, crop_base: 0.02, crop_clamp: 0.10, noise_sigma: 0.0, tier: 0, rotation: 0.015, brightness: 0.02 },
        "hard" => RParams { threshold: 12, scale1: 0.05, scale2: 0.04, crop_base: 0.045, crop_clamp: 0.22, noise_sigma: 2.55, tier: 1, rotation: 0.05, brightness: 0.045 },
        "extreme" => RParams { threshold: 14, scale1: 0.07, scale2: 0.08, crop_base: 0.06, crop_clamp: 0.28, noise_sigma: 5.10, tier: 2, rotation: 0.07, brightness: 0.06 },
        _ => RParams { threshold: 12, scale1: 0.03, scale2: 0.0, crop_base: 0.035, crop_clamp: 0.18, noise_sigma: 0.0, tier: 0, rotation: 0.03, brightness: 0.035 },
    }
}

const IMG_EXTS: [&str; 6] = ["jpg", "jpeg", "png", "webp", "heic", "heif"];

fn ext_of(p: &str) -> String {
    Path::new(p).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

fn dhash(img: &RgbImage) -> u64 {
    let gray: GrayImage = DynamicImage::ImageRgb8(img.clone()).to_luma8();
    let small = imageops::resize(&gray, 9, 8, imageops::FilterType::Triangle);
    let mut hash: u64 = 0;
    let mut bit = 0;
    for y in 0..8u32 {
        for x in 0..8u32 {
            if small.get_pixel(x, y)[0] > small.get_pixel(x + 1, y)[0] {
                hash |= 1 << bit;
            }
            bit += 1;
        }
    }
    hash
}

fn hamming(a: u64, b: u64) -> u32 { (a ^ b).count_ones() }

fn render_image_copy(src: &RgbImage, rng: &mut Prng, p: &RParams, attempt: u32, mirror: bool) -> RgbImage {
    let (w, h) = (src.width(), src.height());
    let sx = 1.0 + rng.symmetric(p.scale1);
    let sy = if p.scale2 > 0.0 { 1.0 + rng.symmetric(p.scale2) } else { sx };
    let nw = ((w as f64 * sx).round() as u32).max(2);
    let nh = ((h as f64 * sy).round() as u32).max(2);
    let mut img = imageops::resize(src, nw, nh, imageops::FilterType::Lanczos3);

    // crop escalates: base + (attempt-1)*0.007, clamped to crop_clamp
    let frac = (p.crop_base + (attempt as f64 - 1.0) * 0.007).min(p.crop_clamp);
    let cx = (img.width() as f64 * frac) as u32;
    let cy = (img.height() as f64 * frac) as u32;
    if cx >= 1 && cy >= 1 && img.width() > cx * 2 + 1 && img.height() > cy * 2 + 1 {
        img = imageops::crop_imm(&img, cx, cy, img.width() - cx * 2, img.height() - cy * 2).to_image();
    }

    let _ = p.rotation; // reddit image path is geometry + sharpen/blur + noise only (no brightness)

    // sharpen/blur tier
    match p.tier {
        1 => img = imageops::unsharpen(&img, 1.0, 2),
        2 => {
            let sigma = 0.4 + rng.next_f64() * 0.4;
            img = imageops::blur(&img, sigma as f32);
        }
        _ => {}
    }

    // Gaussian noise (Box–Muller) at σ = noise_sigma
    if p.noise_sigma > 0.0 {
        for px in img.pixels_mut() {
            let u1 = rng.next_f64().max(1e-12);
            let u2 = rng.next_f64();
            let n = ((-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos() * p.noise_sigma).round() as i32;
            for c in 0..3 {
                px[c] = (px[c] as i32 + n).clamp(0, 255) as u8;
            }
        }
    }

    if mirror {
        imageops::flip_horizontal_in_place(&mut img);
    }
    img
}

fn write_jpeg(img: &RgbImage, path: &Path, q: u8) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::new(file);
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, q);
    enc.encode(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())
}

fn render_image(input: &str, cfg: &RedditSpoofConfig, out_dir: &Path, base_name: &str, cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(i64), on_output: &mut dyn FnMut(&Path)) -> Result<(), String> {
    let src = image::open(input).map_err(|e| e.to_string())?.to_rgb8();
    let p = rparams(&cfg.strength);
    let count = cfg.copies.max(1);
    let base_seed = Prng::fnv1a(base_name.as_bytes());

    // reference dhash starts at the source, then updates to each accepted output
    let mut prev_hash = dhash(&src);

    for idx in 0..count {
        if cancel.load(Ordering::Relaxed) { return Err("Cancelled by user".into()); }
        let mut rng = Prng::per_output(base_seed, idx as u64);

        // up to 14 attempts; accept first candidate with hamming(candidate, previous) >= threshold, else keep best
        let mut best: Option<RgbImage> = None;
        let mut best_dist = 0u32;
        for attempt in 1..=14u32 {
            let cand = render_image_copy(&src, &mut rng, &p, attempt, cfg.mirror);
            let dist = hamming(dhash(&cand), prev_hash);
            if dist >= p.threshold {
                best = Some(cand);
                break;
            }
            if best.is_none() || dist > best_dist {
                best_dist = dist;
                best = Some(cand);
            }
        }
        let best = best.unwrap();
        prev_hash = dhash(&best); // chain: reference becomes this accepted output

        let q = 88 + (rng.next_u64() % 9) as u8; // [88,96]
        let tag = (rng.next_u64() % 10000) as u32; // random 4-digit tag
        let stem = format!("{base_name}_img_{:03}_{:04}", idx + 1, tag);
        let out_path = storage::unique_output_path(out_dir, &stem, "jpg");
        write_jpeg(&best, &out_path, q)?;
        if cfg.exif.mode != "off" {
            if let Some(device) = exif::resolve_device(&cfg.exif, &mut rng) {
                let dt = exif::random_datetime(&mut rng);
                let _ = exif::inject_exif(&out_path, &device, &dt, best.width(), best.height());
            }
        }
        on_output(&out_path);
        on_progress(((idx as i64 + 1) * 100 / count as i64).clamp(0, 100));
    }
    on_progress(100);
    Ok(())
}

fn render_video(input: &str, cfg: &RedditSpoofConfig, out_dir: &Path, base_name: &str, cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(i64), on_output: &mut dyn FnMut(&Path)) -> Result<(), String> {
    ffmpeg::ensure_ffmpeg()?;
    let probe = ffmpeg::probe_video(input).unwrap_or(crate::models::ProbeResult { width: 0, height: 0, duration_secs: 0.0 });
    let has_audio = ffmpeg::has_audio(input);
    let p = rparams(&cfg.strength);
    let count = cfg.copies.max(1);
    let base_seed = Prng::fnv1a(base_name.as_bytes());

    for idx in 0..count {
        if cancel.load(Ordering::Relaxed) { return Err("Cancelled by user".into()); }
        let mut rng = Prng::per_output(base_seed, idx as u64);
        let mut vf: Vec<String> = Vec::new();
        let ang = rng.symmetric(p.rotation * 30.0); // small degrees
        if ang.abs() > 0.01 { vf.push(format!("rotate={:.6}:ow=iw:oh=ih:c=black:bilinear=1", ang * PI / 180.0)); }
        let f = 1.0 + p.crop_base + rng.next_f64() * 0.03;
        let panx = rng.next_f64(); let pany = rng.next_f64();
        vf.push(format!("scale=iw*{f:.5}:ih*{f:.5},crop=in_w/{f:.5}:in_h/{f:.5}:(in_w-out_w)*{panx:.4}:(in_h-out_h)*{pany:.4}"));
        vf.push(format!("eq=brightness={:.4}", rng.symmetric(p.brightness)));
        if p.noise_sigma > 0.0 {
            let n = (p.noise_sigma).round() as i64;
            vf.push(format!("noise=c0s={n}:c0f=t+u:c1s={n}:c1f=t+u:c2s={n}"));
        }
        if cfg.mirror { vf.push("hflip".into()); }
        vf.push("scale=trunc(iw/2)*2:trunc(ih/2)*2".into());

        let day = rng.int_range(1, 27);
        let hh = rng.int_range(0, 23);
        let mm = rng.int_range(0, 59);
        let ss = rng.int_range(0, 59);
        let tag = (rng.next_u64() % 10000) as u32;
        let stem = format!("{base_name}_vid_{:03}_{:04}", idx + 1, tag);
        let out_path = storage::unique_output_path(out_dir, &stem, "mp4");
        let mut args: Vec<String> = vec!["-i".into(), input.into(), "-vf".into(), vf.join(",")];
        args.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "23"].map(String::from));
        if has_audio { args.extend(["-c:a", "aac", "-b:a", "192k"].map(String::from)); } else { args.push("-an".into()); }
        args.extend(["-bsf:v", "filter_units=remove_types=6"].map(String::from));
        args.extend(["-flags:v", "+bitexact"].map(String::from));
        args.extend(["-map_metadata", "-1", "-map_chapters", "-1", "-metadata"].map(String::from));
        args.push(format!("creation_time=2026-05-{day:02} {hh:02}:{mm:02}:{ss:02}"));
        args.extend(["-metadata:s:v:0", "handler_name=Core Media Video"].map(String::from));
        if has_audio { args.extend(["-metadata:s:a:0", "handler_name=Core Media Audio"].map(String::from)); }
        args.extend(["-movflags", "+faststart"].map(String::from));
        args.push(out_path.to_string_lossy().to_string());

        let idx_i = idx as i64; let count_i = count as i64;
        ffmpeg::run_with_progress(&args, probe.duration_secs.max(0.1), cancel, |pct| {
            on_progress(((idx_i * 100 + pct) / count_i).clamp(0, 99));
        })?;
        on_output(&out_path);
        on_progress(((idx_i + 1) * 100 / count_i).clamp(0, 100));
    }
    on_progress(100);
    Ok(())
}

pub fn render(input: &str, cfg: &RedditSpoofConfig, out_dir: &Path, base_name: &str, cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(i64), on_output: &mut dyn FnMut(&Path)) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    if IMG_EXTS.contains(&ext_of(input).as_str()) {
        render_image(input, cfg, out_dir, base_name, cancel, on_progress, on_output)
    } else {
        render_video(input, cfg, out_dir, base_name, cancel, on_progress, on_output)
    }
}
