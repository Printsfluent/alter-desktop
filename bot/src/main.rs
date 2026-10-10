// Alter Telegram bot: approved users send a video, the bot runs the same
// spoof pipeline as the desktop app and replies with the spoofed copies.
#[path = "../../src-tauri/src/ffmpeg.rs"]
#[allow(dead_code)]
mod ffmpeg;
#[path = "../../src-tauri/src/exif.rs"]
#[allow(dead_code)]
mod exif;
#[path = "../../src-tauri/src/spoof_image.rs"]
mod spoof_image;
#[path = "../../src-tauri/src/models.rs"]
#[allow(dead_code)]
mod models;
#[path = "../../src-tauri/src/prng.rs"]
#[allow(dead_code)]
mod prng;
#[path = "../../src-tauri/src/spoof_video.rs"]
mod spoof_video;
mod storage;
#[path = "../../src-tauri/src/strength.rs"]
mod strength;
mod telegram;

use models::{ExifConfig, ImageSpoofConfig, SpoofConfig};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use telegram::Telegram;

// Bot API hard limits for cloud-hosted bots.
const MAX_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;
const MAX_UPLOAD_BYTES: u64 = 50 * 1024 * 1024;
const VIDEO_EXTS: [&str; 6] = ["mp4", "mov", "m4v", "mkv", "webm", "avi"];
const IMAGE_EXTS: [&str; 6] = ["jpg", "jpeg", "png", "webp", "tif", "tiff"];

struct Job {
    chat: i64,
    message_id: i64,
    file_id: String,
    file_name: String,
    is_image: bool,
    copies: u32,
    mirror: bool,
}

#[derive(Clone, Copy)]
enum PendingStep { Copies, TextCheck { copies: u32 } }

struct PendingMedia {
    message_id: i64,
    file_id: String,
    file_name: String,
    is_image: bool,
    step: PendingStep,
}

struct Allowlist {
    ids: HashSet<i64>,
    usernames: HashSet<String>,
}

impl Allowlist {
    fn from_env() -> Self {
        let mut ids = HashSet::new();
        let mut usernames = HashSet::new();
        for raw in std::env::var("ALLOWED_USERS").unwrap_or_default().split(',') {
            let entry = raw.trim();
            if entry.is_empty() {
                continue;
            }
            match entry.parse::<i64>() {
                Ok(id) => {
                    ids.insert(id);
                }
                Err(_) => {
                    usernames.insert(entry.trim_start_matches('@').to_lowercase());
                }
            }
        }
        Self { ids, usernames }
    }

    fn is_empty(&self) -> bool {
        self.ids.is_empty() && self.usernames.is_empty()
    }

    fn allows(&self, from: &Value) -> bool {
        if let Some(id) = from["id"].as_i64() {
            if self.ids.contains(&id) {
                return true;
            }
        }
        from["username"]
            .as_str()
            .map(|u| self.usernames.contains(&u.to_lowercase()))
            .unwrap_or(false)
    }
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

fn spoof_config_from_env() -> SpoofConfig {
    let strength = std::env::var("SPOOF_STRENGTH").unwrap_or_else(|_| "medium".into()).to_lowercase();
    let strength = match strength.as_str() {
        "minimal" | "medium" | "hard" | "extreme" => strength,
        _ => "medium".into(),
    };
    SpoofConfig {
        strength,
        outputs_per_source: env_or("SPOOF_COPIES", 1u32).clamp(1, 5),
        video_spoof: true,
        speed_spoof: env_or("SPOOF_SPEED", true),
        bw_pct: env_or("SPOOF_BW_PCT", 0u32).min(100),
        mirror_pct: env_or("SPOOF_MIRROR_PCT", 0u32).min(100),
        trim_secs: env_or("SPOOF_TRIM_SECS", 0.0f64).max(0.0),
        trim_start: true,
        trim_end: true,
    }
}

fn video_in(msg: &Value) -> Option<(String, String, u64)> {
    let size = |v: &Value| v["file_size"].as_u64().unwrap_or(0);
    let video = &msg["video"];
    if let Some(id) = video["file_id"].as_str() {
        let name = video["file_name"].as_str().unwrap_or("video.mp4").to_string();
        return Some((id.to_string(), name, size(video)));
    }
    let doc = &msg["document"];
    let id = doc["file_id"].as_str()?;
    let name = doc["file_name"].as_str().unwrap_or("video.mp4").to_string();
    let mime_ok = doc["mime_type"].as_str().map(|m| m.starts_with("video/")).unwrap_or(false);
    let ext = Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if mime_ok || VIDEO_EXTS.contains(&ext.as_str()) {
        return Some((id.to_string(), name, size(doc)));
    }
    None
}

fn media_in(msg: &Value) -> Option<(String, String, u64, bool)> {
    if let Some((id, name, size)) = video_in(msg) {
        return Some((id, name, size, false));
    }
    let doc = &msg["document"];
    let id = doc["file_id"].as_str()?;
    let name = doc["file_name"].as_str().unwrap_or("image.jpg").to_string();
    let mime_ok = doc["mime_type"].as_str().map(|m| ["image/jpeg", "image/png", "image/webp", "image/tiff"].contains(&m)).unwrap_or(false);
    let ext = Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if mime_ok || IMAGE_EXTS.contains(&ext.as_str()) {
        return Some((id.to_string(), name, doc["file_size"].as_u64().unwrap_or(0), true));
    }
    // Telegram photos arrive as multiple sizes; choose the largest.
    let photo = msg["photo"].as_array()?.last()?;
    Some((photo["file_id"].as_str()?.to_string(), "image.jpg".into(), photo["file_size"].as_u64().unwrap_or(0), true))
}

fn input_ext(name: &str) -> String {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if VIDEO_EXTS.contains(&ext.as_str()) { ext } else { "mp4".into() }
}

fn image_ext(name: &str) -> String {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ["jpg", "jpeg", "png", "webp", "tif", "tiff"].contains(&ext.as_str()) { ext } else { "jpg".into() }
}

fn image_config_from_env(copies: u32) -> ImageSpoofConfig {
    ImageSpoofConfig {
        copies: copies.clamp(1, 5),
        exif: ExifConfig { mode: "random".into(), pool: "all".into(), device_id: None },
    }
}

fn process(tg: &Telegram, cfg: &SpoofConfig, job: &Job) -> Result<(), String> {
    let work = std::env::temp_dir().join("alter-bot").join(uuid::Uuid::new_v4().simple().to_string());
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let result = (|| {
        let input = work.join(format!("input.{}", if job.is_image { image_ext(&job.file_name) } else { input_ext(&job.file_name) }));
        let media_kind = if job.is_image { "image" } else { "video" };
        println!("job start: chat={} type={} copies={} mirror={}", job.chat, media_kind, job.copies, job.mirror);
        let status_message_id = tg.send_text_with_id(job.chat, Some(job.message_id), &format!("Starting: downloading your {media_kind}…"));
        tg.typing_video(job.chat);
        tg.download(&job.file_id, &input)?;
        let input_size = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
        println!("job download complete: chat={} bytes={input_size}", job.chat);
        if let Some(status_id) = status_message_id {
            tg.edit_text(job.chat, status_id, &format!("Download complete. Rendering {} variant(s): 0%", job.copies));
        }

        let base = format!("alter_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
        let out_dir = work.join("out");
        let cancel = Arc::new(AtomicBool::new(false));
        let mut outputs: Vec<PathBuf> = Vec::new();
        let mut reported_progress = 0i64;
        let mut on_progress = |pct: i64| {
            let pct = pct.clamp(0, 100);
            if pct >= reported_progress + 25 {
                reported_progress = (pct / 25) * 25;
                println!("job render progress: chat={} percent={pct}", job.chat);
                if let Some(status_id) = status_message_id {
                    tg.edit_text(job.chat, status_id, &format!("Rendering: {pct}% complete."));
                }
            }
        };
        let mut on_output = |p: &Path| outputs.push(p.to_path_buf());
        if job.is_image {
            spoof_image::render_with_mirror(&input.to_string_lossy(), &image_config_from_env(job.copies), &out_dir, &base, &cancel, job.mirror, &mut on_progress, &mut on_output)?;
        } else {
            let mut job_cfg = cfg.clone();
            job_cfg.outputs_per_source = job.copies;
            job_cfg.mirror_pct = if job.mirror { 100 } else { 0 };
            spoof_video::render(&input.to_string_lossy(), &job_cfg, &out_dir, &base, &cancel, &mut on_progress, &mut on_output)?;
        }

        let total = outputs.len();
        println!("job render complete: chat={} outputs={total}", job.chat);
        if let Some(status_id) = status_message_id {
            tg.edit_text(job.chat, status_id, &format!("Rendering complete. Sending {total} output(s)…"));
        }
        for (i, path) in outputs.iter().enumerate() {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            if size > MAX_UPLOAD_BYTES {
                tg.send_text(
                    job.chat,
                    Some(job.message_id),
                    &format!(
                        "Copy {} is {} MB, over Telegram's 50 MB bot upload limit. Send a shorter clip.",
                        i + 1,
                        size / (1024 * 1024)
                    ),
                );
                continue;
            }
            tg.typing_video(job.chat);
            let caption = if total > 1 { format!("Spoofed copy {}/{}", i + 1, total) } else { "Spoofed".into() };
            println!("job upload started: chat={} copy={}/{} bytes={size}", job.chat, i + 1, total);
            if job.is_image { tg.send_document(job.chat, job.message_id, path, &caption)?; }
            else { tg.send_video(job.chat, job.message_id, path, &caption)?; }
            println!("job upload complete: chat={} copy={}/{}", job.chat, i + 1, total);
        }
        if let Some(status_id) = status_message_id {
            tg.edit_text(job.chat, status_id, "All variants sent.");
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

fn spawn_health_server() {
    let Ok(port) = std::env::var("PORT") else { return };
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        let listener = match std::net::TcpListener::bind(format!("0.0.0.0:{port}")) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("health server could not bind {port}: {e}");
                return;
            }
        };
        for mut stream in listener.incoming().flatten() {
            let mut buf = [0u8; 512];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        }
    });
}

const HELP: &str = "Send me an Instagram/Threads video or image (up to 20 MB). I'll ask how many variants you want, then whether the media contains written text. Mirroring is enabled only when you confirm there is no text.\n\nDefault video strength: Medium. Send images as a File to preserve their original quality. Use /cancel to cancel a pending upload or /id to see your Telegram ID.";

fn main() {
    let token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    if token.trim().is_empty() {
        eprintln!("TELEGRAM_BOT_TOKEN is not set");
        std::process::exit(1);
    }
    if let Err(e) = ffmpeg::ensure_ffmpeg() {
        eprintln!("{e}");
        std::process::exit(1);
    }

    let allow = Allowlist::from_env();
    if allow.is_empty() {
        eprintln!("ALLOWED_USERS is empty: every request will be refused until you add users.");
    }
    let cfg = spoof_config_from_env();
    println!(
        "spoof preset: strength={} copies={} speed={} mirror={}% bw={}% trim={}s",
        cfg.strength, cfg.outputs_per_source, cfg.speed_spoof, cfg.mirror_pct, cfg.bw_pct, cfg.trim_secs
    );

    spawn_health_server();

    let tg = Arc::new(Telegram::new(token.trim()));
    match tg.me() {
        Ok(me) => println!("running as @{}", me["username"].as_str().unwrap_or("?")),
        Err(e) => {
            eprintln!("Telegram rejected the token: {e}");
            std::process::exit(1);
        }
    }
    if let Err(e) = tg.delete_webhook() {
        eprintln!("deleteWebhook failed: {e}");
    }
    match tg.configure_command_menu() {
        Ok(()) => println!("Telegram command menu configured"),
        Err(e) => eprintln!("Telegram command menu setup failed: {e}"),
    }

    // Jobs run one at a time; small Render instances can't encode in parallel.
    let queued = Arc::new(AtomicUsize::new(0));
    let mut pending: HashMap<(i64, i64), PendingMedia> = HashMap::new();
    let (tx, rx) = mpsc::channel::<Job>();
    {
        let tg = Arc::clone(&tg);
        let queued = Arc::clone(&queued);
        std::thread::spawn(move || {
            for job in rx {
                if let Err(e) = process(&tg, &cfg, &job) {
                    eprintln!("job for chat {} failed: {e}", job.chat);
                    tg.send_text(job.chat, Some(job.message_id), &format!("Spoof failed: {e}"));
                }
                queued.fetch_sub(1, Ordering::SeqCst);
            }
        });
    }

    let mut offset = 0i64;
    loop {
        let updates = match tg.updates(offset) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("getUpdates: {e}");
                std::thread::sleep(Duration::from_secs(3));
                continue;
            }
        };
        for update in updates {
            if let Some(id) = update["update_id"].as_i64() {
                offset = offset.max(id + 1);
                println!("telegram update received: {id}");
            }
            let msg = &update["message"];
            let (Some(chat), Some(message_id)) = (msg["chat"]["id"].as_i64(), msg["message_id"].as_i64()) else {
                continue;
            };
            let from = &msg["from"];
            let text = msg["text"].as_str().unwrap_or("").trim();

            if text.starts_with("/id") || text.eq_ignore_ascii_case("ID") {
                let uid = from["id"].as_i64().unwrap_or(0);
                tg.send_text(chat, Some(message_id), &format!("Your Telegram ID is {uid}"));
                continue;
            }
            if !allow.allows(from) {
                let uid = from["id"].as_i64().unwrap_or(0);
                tg.send_text(
                    chat,
                    Some(message_id),
                    &format!("You're not on this bot's approved list. Send the admin your Telegram ID: {uid}"),
                );
                continue;
            }
            let owner_id = from["id"].as_i64().unwrap_or(0);
            let pending_key = (chat, owner_id);
            if text.starts_with("/start") || text.starts_with("/help")
                || text.eq_ignore_ascii_case("START") || text.eq_ignore_ascii_case("HELP")
            {
                tg.send_help_with_keyboard(chat, HELP);
                continue;
            }

            if text.starts_with("/cancel") {
                if pending.remove(&pending_key).is_some() {
                    tg.send_text(chat, Some(message_id), "Cancelled. Send another media file when you're ready.");
                } else {
                    tg.send_text(chat, Some(message_id), "There's no pending media to cancel.");
                }
                continue;
            }

            if let Some(flow) = pending.get_mut(&pending_key) {
                match flow.step {
                    PendingStep::Copies => {
                        let copies = text.parse::<u32>().ok().filter(|n| (1..=5).contains(n));
                        let Some(copies) = copies else {
                            tg.send_text(chat, Some(message_id), "Reply with the number of variants you want (1–5), or /cancel.");
                            continue;
                        };
                        flow.step = PendingStep::TextCheck { copies };
                        tg.send_text(chat, Some(message_id), "Does the media have written captions or other text overlaid on it? Reply yes or no. If yes, flipping will be disabled.");
                    }
                    PendingStep::TextCheck { copies } => {
                        let answer = text.to_lowercase();
                        let mirror = match answer.as_str() {
                            "yes" | "y" | "text" | "has text" | "captions" => false,
                            "no" | "n" | "none" | "no text" | "no captions" => true,
                            _ => {
                                tg.send_text(chat, Some(message_id), "Reply yes if there is written text, or no if there is none. Flipping stays disabled until you confirm no text.");
                                continue;
                            }
                        };
                        let flow = pending.remove(&pending_key).unwrap();
                        let ahead = queued.fetch_add(1, Ordering::SeqCst);
                        let note = if ahead == 0 {
                            format!("Thanks. I'll make {copies} variant(s){} now…", if mirror { " and mirror them" } else { "; mirroring is off" })
                        } else {
                            format!("Thanks. {ahead} file(s) are ahead of yours; I'll make {copies} variant(s) when it's your turn.")
                        };
                        tg.send_text(chat, Some(message_id), &note);
                        let _ = tx.send(Job { chat, message_id: flow.message_id, file_id: flow.file_id, file_name: flow.file_name, is_image: flow.is_image, copies, mirror });
                    }
                }
                continue;
            }

            let Some((file_id, file_name, size, is_image)) = media_in(msg) else {
                if !text.is_empty() || msg["photo"].is_array() || msg["document"].is_object() {
                    tg.send_text(chat, Some(message_id), "Send an Instagram/Threads video or image file.");
                }
                continue;
            };
            if size > MAX_DOWNLOAD_BYTES {
                tg.send_text(
                    chat,
                    Some(message_id),
                    &format!(
                        "That media file is {} MB. Telegram only lets bots download files up to 20 MB, so trim or compress it first.",
                        size / (1024 * 1024)
                    ),
                );
                continue;
            }
            pending.insert(pending_key, PendingMedia { message_id, file_id, file_name, is_image, step: PendingStep::Copies });
            println!("media queued for configuration: chat={} type={} bytes={size}", chat, if is_image { "image" } else { "video" });
            tg.send_text(chat, Some(message_id), "How many variants do you want? Reply with a number from 1 to 5.");
        }
    }
}
