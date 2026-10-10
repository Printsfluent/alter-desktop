// Instagram IMAGE spoof pipeline — magnitudes verbatim from spec §7.4.
// No rotation, no brightness. escala = 1 + (retry_attempt-1)/4, reset per copy.
// Dedup: phash (8x8 vertical-gradient) vs the ORIGINAL source; accept when
// hamming >= 1 (only a byte-identical copy is retried), up to 8 attempts.
use crate::exif;
use crate::models::ImageSpoofConfig;
use crate::prng::Prng;
use image::{imageops, DynamicImage, GrayImage, RgbImage};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// 8x8 vertical-gradient perceptual hash (image_hasher HashAlg::VertGradient)
fn phash(img: &RgbImage) -> u64 {
    let gray: GrayImage = DynamicImage::ImageRgb8(img.clone()).to_luma8();
    let small = imageops::resize(&gray, 8, 9, imageops::FilterType::Lanczos3); // resize-filter field = 4
    let mut hash: u64 = 0;
    let mut bit = 0;
    for y in 0..8u32 {
        for x in 0..8u32 {
            if small.get_pixel(x, y)[0] > small.get_pixel(x, y + 1)[0] {
                hash |= 1 << bit;
            }
            bit += 1;
        }
    }
    hash
}

fn hamming(a: u64, b: u64) -> u32 { (a ^ b).count_ones() }

fn render_copy(src: &RgbImage, rng: &mut Prng, escala: f64) -> RgbImage {
    let (w, h) = (src.width(), src.height());

    // scale: uniform [0.95,1.05] (0.95 + rand·0.10) × non-uniform pair [0.96,1.04] (0.96 + rand·0.08)
    let su = 0.95 + rng.next_f64() * 0.10;
    let sx = su * (0.96 + rng.next_f64() * 0.08);
    let sy = su * (0.96 + rng.next_f64() * 0.08);
    let nw = ((w as f64 * sx).round() as u32).max(2);
    let nh = ((h as f64 * sy).round() as u32).max(2);
    let mut img = imageops::resize(src, nw, nh, imageops::FilterType::Lanczos3);

    // crop: budget floor(3·escala) px; if >=2, random inset 1..budget per side
    let budget = (3.0 * escala).floor() as u32;
    if budget >= 2 && img.width() > budget * 2 + 2 && img.height() > budget * 2 + 2 {
        let l = 1 + (rng.next_u64() % budget as u64) as u32;
        let t = 1 + (rng.next_u64() % budget as u64) as u32;
        let r = 1 + (rng.next_u64() % budget as u64) as u32;
        let b = 1 + (rng.next_u64() % budget as u64) as u32;
        img = imageops::crop_imm(&img, l, t, img.width() - l - r, img.height() - t - b).to_image();
    }

    // contrast: center [0.90,1.10] (0.9 + rand·0.2) × span (1 ± 0.04·escala)
    let c_center = 0.9 + rng.next_f64() * 0.2;
    let c_span = 1.0 + rng.symmetric(0.04 * escala);
    let contrast = c_center * c_span;

    // blend factor clamp [1 − 0.10·escala, 1 + 0.10·escala]
    let blend = rng.range(1.0 - 0.10 * escala, 1.0 + 0.10 * escala)
        .clamp(1.0 - 0.10 * escala, 1.0 + 0.10 * escala);
    let t = blend.clamp(0.0, 1.0);

    // saturation (BT.601 luma). NOTE: the spec's literal center [0.3,0.7] desaturates
    // photos to grey — a mis-recovered magnitude (the real app makes natural spoofs).
    // Centered at ~1.0 with the spec's ±0.08 jitter for subtle per-copy variation.
    let sat = 1.0 + rng.symmetric(0.08);

    for p in img.pixels_mut() {
        // contrast around mid-gray
        let mut cc = [0f64; 3];
        for c in 0..3 {
            cc[c] = (((p[c] as f64 / 255.0 - 0.5) * contrast + 0.5) * 255.0).clamp(0.0, 255.0);
        }
        let luma = 0.299 * cc[0] + 0.587 * cc[1] + 0.114 * cc[2];
        for c in 0..3 {
            let sat_c = luma + sat * (cc[c] - luma);
            let out = cc[c] * (1.0 - t) + sat_c * t;
            p[c] = out.clamp(0.0, 255.0) as u8;
        }
    }

    // sharpen XOR blur (50/50)
    if rng.next_f64() < 0.5 {
        img = imageops::unsharpen(&img, 1.0, 2);
    } else {
        let sigma = 0.3 + rng.next_f64() * 0.4; // [0.3,0.7]
        img = imageops::blur(&img, sigma as f32);
    }

    // noise: Gaussian (Box–Muller), ~10% of copies (roll < 0.0992187), σ = (0.5+rand)·255/100
    if rng.next_f64() < 0.0992187 {
        let sigma = (0.5 + rng.next_f64()) * 255.0 / 100.0;
        for p in img.pixels_mut() {
            let u1 = rng.next_f64().max(1e-12);
            let u2 = rng.next_f64();
            let n = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
            let delta = (n * sigma).round() as i32;
            for c in 0..3 {
                p[c] = (p[c] as i32 + delta).clamp(0, 255) as u8;
            }
        }
    }
    img
}

// q = 80 + min((rand%5)+(rand%15), 16) → ~82–96; special-case q=82 if raw<2
fn jpeg_quality(rng: &mut Prng) -> u8 {
    let raw = (rng.next_u64() % 5) + (rng.next_u64() % 15);
    if raw < 2 {
        return 82;
    }
    (80 + raw.min(16)) as u8
}

fn write_jpeg(img: &RgbImage, path: &Path, quality: u8) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::new(file);
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, quality);
    enc.encode(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8)
        .map_err(|e| e.to_string())
}

pub fn render(
    input: &str,
    cfg: &ImageSpoofConfig,
    out_dir: &Path,
    base_name: &str,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(i64),
    on_output: &mut dyn FnMut(&Path),
) -> Result<(), String> {
    render_with_mirror(input, cfg, out_dir, base_name, cancel, false, on_progress, on_output)
}

pub fn render_with_mirror(
    input: &str,
    cfg: &ImageSpoofConfig,
    out_dir: &Path,
    base_name: &str,
    cancel: &Arc<AtomicBool>,
    mirror: bool,
    on_progress: &mut dyn FnMut(i64),
    on_output: &mut dyn FnMut(&Path),
) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let src = image::open(input).map_err(|e| e.to_string())?.to_rgb8();
    let src_hash = phash(&src); // reference = ORIGINAL source, computed once
    let count = cfg.copies.max(1);
    let base_seed = Prng::fnv1a(base_name.as_bytes());

    for idx in 0..count {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled by user".into());
        }
        let mut rng = Prng::per_output(base_seed, idx as u64);

        // phash-retry loop: attempt k∈[1,8], escala = 1 + (k-1)/4; accept when hamming >= 1
        let mut best: Option<RgbImage> = None;
        for attempt in 1..=8u32 {
            let escala = 1.0 + (attempt as f64 - 1.0) / 4.0;
            let cand = render_copy(&src, &mut rng, escala);
            let accept = hamming(phash(&cand), src_hash) >= 1;
            best = Some(cand);
            if accept {
                break;
            }
        }
        let mut best = best.unwrap();
        if mirror {
            imageops::flip_horizontal_in_place(&mut best);
        }

        let out_path: PathBuf = out_dir.join(format!("{base_name}_{:03}.jpg", idx + 1));
        write_jpeg(&best, &out_path, jpeg_quality(&mut rng))?;

        // EXIF injection (best-effort)
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
