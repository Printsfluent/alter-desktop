// End-to-end pipeline checks: exercise the real ffmpeg/image spoof engines.
use oopsie_lib::models::{ExifConfig, ImageSpoofConfig, RedditSpoofConfig, SpoofConfig};
use oopsie_lib::{reddit_spoof, spoof_image, spoof_video};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn scratch() -> std::path::PathBuf {
    let d = std::env::temp_dir().join("oopsie-test");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn make_test_video(path: &Path) {
    // 2s 320x240 test pattern with a tone, via ffmpeg on PATH
    let status = Command::new("ffmpeg")
        .args([
            "-y", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=15:duration=2",
            "-f", "lavfi", "-i", "sine=frequency=440:duration=2",
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest",
            path.to_str().unwrap(),
        ])
        .status()
        .expect("ffmpeg spawn");
    assert!(status.success(), "failed to create test video");
}

fn make_test_image(path: &Path) {
    let status = Command::new("ffmpeg")
        .args([
            "-y", "-f", "lavfi", "-i", "testsrc=size=640x480:rate=1:duration=1",
            "-frames:v", "1", path.to_str().unwrap(),
        ])
        .status()
        .expect("ffmpeg spawn");
    assert!(status.success(), "failed to create test image");
}

#[test]
fn video_spoof_produces_outputs() {
    let dir = scratch();
    let input = dir.join("in.mp4");
    make_test_video(&input);
    let out_dir = dir.join("vid_out");
    let _ = std::fs::remove_dir_all(&out_dir);

    let cfg = SpoofConfig {
        strength: "medium".into(),
        outputs_per_source: 3,
        video_spoof: true,
        speed_spoof: true,
        bw_pct: 0,
        mirror_pct: 50,
        trim_secs: 0.0,
        trim_start: true,
        trim_end: true,
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let mut last = 0;
    let mut n_out = 0;
    let res = spoof_video::render(
        input.to_str().unwrap(),
        &cfg,
        &out_dir,
        "clip",
        &cancel,
        &mut |p| last = p,
        &mut |_p| n_out += 1,
    );
    assert!(res.is_ok(), "video spoof failed: {:?}", res);
    assert_eq!(n_out, 3, "expected 3 outputs");
    assert_eq!(last, 100);
    for i in 1..=3 {
        let f = out_dir.join(format!("clip_{:03}.mp4", i));
        assert!(f.exists(), "missing {:?}", f);
        assert!(std::fs::metadata(&f).unwrap().len() > 0, "empty {:?}", f);
    }

    // verify the injected output metadata is actually written
    let tags = ffprobe_tags(&out_dir.join("clip_001.mp4"));
    assert!(tags.contains("creation_time"), "creation_time not written: {tags}");
    assert!(tags.contains("2026-05"), "creation_time year/month wrong: {tags}");
    assert!(tags.contains("Core Media Video"), "handler_name not written: {tags}");
    assert!(tags.contains("vicut"), "ViCut watermark comment not written: {tags}");
}

fn ffprobe_tags(path: &Path) -> String {
    let out = Command::new("ffprobe")
        .args([
            "-v", "error", "-show_entries",
            "format_tags:stream_tags", "-of", "json", path.to_str().unwrap(),
        ])
        .output()
        .expect("ffprobe spawn");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn video_spoof_extreme_exercises_pitch_and_pad() {
    let dir = scratch();
    let input = dir.join("in.mp4");
    make_test_video(&input);
    let out_dir = dir.join("vid_ext");
    let _ = std::fs::remove_dir_all(&out_dir);

    let cfg = SpoofConfig {
        strength: "extreme".into(),
        outputs_per_source: 2,
        video_spoof: true,
        speed_spoof: true, // enables pitch (asetrate/aresample) on the audio track
        bw_pct: 0,
        mirror_pct: 0,
        trim_secs: 0.0,
        trim_start: true,
        trim_end: true,
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let mut n = 0;
    let res = spoof_video::render(
        input.to_str().unwrap(), &cfg, &out_dir, "ext", &cancel,
        &mut |_p| {}, &mut |_p| n += 1,
    );
    assert!(res.is_ok(), "extreme video spoof failed: {:?}", res);
    assert_eq!(n, 2);
    for i in 1..=2 {
        let f = out_dir.join(format!("ext_{:03}.mp4", i));
        assert!(f.exists() && std::fs::metadata(&f).unwrap().len() > 0);
    }
}

#[test]
fn reddit_image_spoof_chained_dedup() {
    let dir = scratch();
    let input = dir.join("in_r.png");
    make_test_image(&input);
    let out_dir = dir.join("reddit_img");
    let _ = std::fs::remove_dir_all(&out_dir);

    let cfg = RedditSpoofConfig {
        copies: 3,
        strength: "hard".into(), // exercises Gaussian noise + tier-1 sharpen + threshold 12
        mirror: true,
        trim_secs: 0.0,
        trim_start: true,
        trim_end: true,
        exif: ExifConfig { mode: "random".into(), pool: "all".into(), device_id: None },
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let mut n = 0;
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    let res = reddit_spoof::render(
        input.to_str().unwrap(), &cfg, &out_dir, "post", &cancel,
        &mut |_p| {}, &mut |p| { n += 1; paths.push(p.to_path_buf()); },
    );
    assert!(res.is_ok(), "reddit image spoof failed: {:?}", res);
    assert_eq!(n, 3, "expected 3 reddit outputs");
    for p in &paths {
        assert!(p.exists() && std::fs::metadata(p).unwrap().len() > 0);
        // filename shape: post_img_NNN_TTTT.jpg
        let name = p.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("post_img_") && name.ends_with(".jpg"), "bad name {name}");
    }
}

#[test]
fn image_spoof_produces_outputs_with_exif() {
    let dir = scratch();
    let input = dir.join("in.png");
    make_test_image(&input);
    let out_dir = dir.join("img_out");
    let _ = std::fs::remove_dir_all(&out_dir);

    let cfg = ImageSpoofConfig {
        copies: 5,
        exif: ExifConfig { mode: "random".into(), pool: "ios".into(), device_id: None },
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let mut n_out = 0;
    let res = spoof_image::render(
        input.to_str().unwrap(),
        &cfg,
        &out_dir,
        "photo",
        &cancel,
        &mut |_p| {},
        &mut |_p| n_out += 1,
    );
    assert!(res.is_ok(), "image spoof failed: {:?}", res);
    assert_eq!(n_out, 5, "expected 5 copies");
    for i in 1..=5 {
        let f = out_dir.join(format!("photo_{:03}.jpg", i));
        assert!(f.exists(), "missing {:?}", f);
        let bytes = std::fs::read(&f).unwrap();
        assert!(bytes.len() > 0);
        // APP1 EXIF marker present (FFE1) after SOI
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8], "not a JPEG");
        assert!(bytes[2] == 0xFF && bytes[3] == 0xE1, "no EXIF APP1 segment injected");
    }
}
