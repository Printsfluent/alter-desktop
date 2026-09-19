// Instagram VIDEO spoof pipeline (spec §7.1/§7.2): per-strength PRNG-driven
// ffmpeg -vf chain, fixed encode args, metadata/creation_time spoof.
use crate::ffmpeg;
use crate::models::SpoofConfig;
use crate::prng::Prng;
use crate::strength;
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn render(
    input: &str,
    cfg: &SpoofConfig,
    out_dir: &Path,
    base_name: &str,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(i64),
    on_output: &mut dyn FnMut(&Path),
) -> Result<(), String> {
    ffmpeg::ensure_ffmpeg()?;
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let probe = ffmpeg::probe_video(input).unwrap_or(crate::models::ProbeResult { width: 0, height: 0, duration_secs: 0.0 });
    let has_audio = ffmpeg::has_audio(input);
    let sample_rate = if has_audio { ffmpeg::audio_sample_rate(input).unwrap_or(44100) } else { 44100 };
    let count = cfg.outputs_per_source.max(1);
    let sp = strength::params(&cfg.strength);

    let base_seed = Prng::fnv1a(base_name.as_bytes());
    // pick which copies get mirrored / grayscaled by share percentage
    let mirror_n = ((cfg.mirror_pct as f64 / 100.0) * count as f64).round() as u32;
    let bw_n = ((cfg.bw_pct as f64 / 100.0) * count as f64).round() as u32;

    for idx in 0..count {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled by user".into());
        }
        let mut rng = Prng::per_output(base_seed, idx as u64);

        // ---- video filter chain ----
        let mut vf: Vec<String> = Vec::new();

        // rotation
        let ang = rng.symmetric(sp.rotation_amp);
        if ang.abs() > 0.01 {
            let rad = ang * PI / 180.0;
            vf.push(format!("rotate={rad:.6}:ow=iw:oh=ih:c=black:bilinear=1"));
        }
        // zoom + crop (pan)
        let f = rng.range(sp.zoom.0, sp.zoom.1);
        let px = rng.next_f64();
        let py = rng.next_f64();
        vf.push(format!("scale=iw*{f:.5}:ih*{f:.5},crop=in_w/{f:.5}:in_h/{f:.5}:(in_w-out_w)*{px:.4}:(in_h-out_h)*{py:.4}"));
        // color EQ
        let sat = rng.range(sp.saturation.0, sp.saturation.1);
        let bri = rng.symmetric(sp.brightness_static_amp);
        let con = 1.0 + rng.symmetric(sp.contrast_amp);
        vf.push(format!("eq=saturation={sat:.4}:brightness={bri:.4}:contrast={con:.4}"));
        // hue
        let hue = rng.symmetric(sp.hue_amp);
        if hue.abs() > 0.01 {
            vf.push(format!("hue=h={hue:.3}"));
        }
        // brightness flicker — amp (range), freq∈[3,5), phase=u
        if let Some((lo, hi)) = sp.flicker {
            let amp = rng.range(lo, hi);
            let freq = rng.range(3.0, 5.0);
            let phase = rng.next_f64();
            let period = 1.0 / freq;
            vf.push(format!("eq=brightness='{amp:.5}*sin(2*PI*(t/{period:.4})+{phase:.4})':eval=frame"));
        }
        // per-channel noise
        if let Some((lo, hi)) = sp.noise {
            let n = rng.int_range(lo, hi);
            vf.push(format!("noise=c0s={n}:c0f=t+u:c1s={n}:c1f=t+u:c2s={n}:c2f=t+u"));
        }
        // border pad + rescale back (spec §5: pad=iw+<p>:…:color=0x<hex>,scale=iw-<p>)
        if let Some((lo, hi)) = sp.pad_base {
            let base = rng.int_range(lo, hi);
            let p = 2 * base;
            let half = p / 2;
            vf.push(format!("pad=iw+{p}:ih+{p}:{half}:{half}:color=0x000000,scale=iw-{p}:ih-{p}"));
        }
        // speed (setpts)
        let speed = rng.range(sp.speed.0, sp.speed.1);
        if cfg.speed_spoof {
            let inv = 1.0 / speed;
            vf.push(format!("setpts={inv:.5}*PTS"));
        }
        // grayscale share (bw_pct)
        if idx < bw_n {
            vf.push("hue=s=0".into());
        }
        // mirror share
        if idx < mirror_n {
            vf.push("hflip".into());
        }
        // even-dim guard
        vf.push("scale=trunc(iw/2)*2:trunc(ih/2)*2".into());

        // ---- audio filter chain ----
        let mut af: Vec<String> = Vec::new();
        if has_audio {
            let vol = rng.symmetric(sp.volume_amp);
            af.push("highpass=f=30".into());
            af.push("lowpass=f=18000".into());
            af.push(format!("volume={vol:.3}dB"));
            if cfg.speed_spoof {
                let atempo = speed.clamp(0.5, 2.0);
                af.push(format!("atempo={atempo:.5}"));
            }
            // audio pitch (hard/extreme): asetrate=SR*pitch + atempo=1/pitch + aresample
            if let (Some((lo, hi)), true) = (sp.pitch, cfg.speed_spoof) {
                let pitch = rng.range(lo, hi);
                af.push(format!("asetrate={}", (sample_rate as f64 * pitch).round() as i64));
                af.push(format!("atempo={:.5}", (1.0 / pitch).clamp(0.5, 2.0)));
                af.push(format!("aresample={sample_rate}"));
            }
        }

        // ---- metadata / creation_time ----
        // day/time share ONE prng draw across both templates (spec §7.3)
        let day = rng.int_range(1, 27);
        let hh = rng.int_range(0, 23);
        let mm = rng.int_range(0, 59);
        let ss = rng.int_range(0, 59);
        let ctime = format!("creation_time=2026-05-{day:02} {hh:02}:{mm:02}:{ss:02}");
        let ctime04 = format!("creation_time=2026-04-{day:02} {hh:02}:{mm:02}:{ss:02}");

        // ---- assemble args ----
        let out_path: PathBuf = out_dir.join(format!("{base_name}_{:03}.mp4", idx + 1));
        let mut args: Vec<String> = Vec::new();
        let mut total = probe.duration_secs;
        if cfg.trim_secs > 0.0 && cfg.trim_start {
            args.push("-ss".into());
            args.push(format!("{:.3}", cfg.trim_secs));
            total -= cfg.trim_secs;
        }
        args.push("-i".into());
        args.push(input.into());
        if cfg.trim_secs > 0.0 && cfg.trim_end && probe.duration_secs > 0.0 {
            let dur = (probe.duration_secs - cfg.trim_secs * 2.0).max(0.1);
            args.push("-t".into());
            args.push(format!("{dur:.3}"));
            total = dur;
        }
        args.push("-vf".into());
        args.push(vf.join(","));
        if has_audio {
            args.push("-af".into());
            args.push(af.join(","));
        }
        args.extend(["-c:v", "libx264", "-preset", "fast", "-crf", "23"].map(String::from));
        if has_audio {
            args.extend(["-c:a", "aac", "-b:a", "192k"].map(String::from));
        } else {
            args.push("-an".into());
        }
        // strip SEI NAL (H.264 BSF, type 6) + bitexact encode
        args.extend(["-bsf:v", "filter_units=remove_types=6"].map(String::from));
        args.extend(["-flags:v", "+bitexact"].map(String::from));
        // strip all metadata + chapters
        args.extend(["-map_metadata", "-1", "-map_chapters", "-1"].map(String::from));
        // creation_time spoof — both 2026-05 (container) and 2026-04 (video stream) templates emitted
        args.push("-metadata".into());
        args.push(ctime);
        args.push("-metadata:s:v:0".into());
        args.push(ctime04);
        // fake ViCut/CapCut editor watermark
        args.push("-metadata".into());
        args.push(format!("comment={}", vicut_blob(&mut rng)));
        // inject handler names
        args.extend(["-metadata:s:v:0", "handler_name=Core Media Video"].map(String::from));
        if has_audio {
            args.extend(["-metadata:s:a:0", "handler_name=Core Media Audio"].map(String::from));
        }
        args.extend(["-movflags", "+faststart"].map(String::from));
        args.push(out_path.to_string_lossy().to_string());

        let idx_i = idx as i64;
        let count_i = count as i64;
        ffmpeg::run_with_progress(&args, total.max(0.1), cancel, |pct| {
            on_progress(((idx_i * 100 + pct) / count_i).clamp(0, 99));
        })?;
        on_output(&out_path);
        on_progress(((idx_i + 1) * 100 / count_i).clamp(0, 100));
    }
    on_progress(100);
    Ok(())
}

// Fake CapCut/ViCut editor watermark (spec §7.3 / EXIF-ffmpeg section);
// __VIDEO_ID__ substituted with a fresh random id per output.
fn vicut_blob(rng: &mut Prng) -> String {
    let vid = format!("{:016x}", rng.next_u64());
    format!(
        "{{\"data\":{{\"editType\":\"default\",\"infoStickerId\":\"\",\"is_ai_lyric\":0,\"is_aimusic_mv\":0,\"is_use_ai_image_generation\":0,\"is_use_ai_video_generation\":0,\"is_use_aimusic_bgm\":0,\"is_use_aimusic_vocal\":0,\"is_use_graph_chart\":0,\"is_use_jichuang_mode_in_ai_writer\":0,\"is_use_relight\":0,\"is_use_vc_sing_clone\":1,\"is_use_voice_clone\":\"0\",\"motion_blur_cnt\":0,\"musicId\":\"\",\"os\":\"mac\",\"product\":\"vicut\",\"stickerId\":\"\",\"videoEffectId\":\"\",\"videoId\":\"{vid}\",\"videoParams\":{{\"be\":0,\"ef\":0,\"ft\":0,\"ma\":0,\"me\":0,\"mu\":0,\"re\":0,\"sp\":0,\"st\":0,\"te\":0,\"tx\":6,\"v\":0,\"vs\":0}}}},\"source_platform\":\"desktop\",\"source_type\":\"vicut\"}}"
    )
}
