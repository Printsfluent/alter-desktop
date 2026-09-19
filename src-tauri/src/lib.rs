// OOPSIE backend — Tauri command surface + module wiring.
pub mod ass;
pub mod captions;
pub mod converters;
pub mod editor;
pub mod exif;
pub mod ffmpeg;
mod jobs;
pub mod license;
pub mod models;
pub mod prng;
pub mod reddit_spoof;
pub mod spoof_image;
pub mod spoof_video;
mod state;
pub mod storage;
pub mod strength;

use models::*;
use serde::Deserialize;
use serde_json::Value;
use state::AppState;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

// ---------- helpers ----------

fn ensure_licensed() -> Result<(), String> {
    Ok(())
}

fn open_path(app: &AppHandle, path: &str) -> Result<(), String> {
    app.opener().open_path(path.to_string(), None::<&str>).map_err(|e| e.to_string())
}

fn stem(p: &str) -> String {
    Path::new(p).file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string()
}

// ---------- spoofer / jobs ----------

#[tauri::command]
fn stage_file_upload(source_path: String) -> Result<String, String> {
    ensure_licensed()?;
    if !Path::new(&source_path).exists() {
        return Err("Source file not found".into());
    }
    let _ = std::fs::create_dir_all(storage::uploads_dir());
    let name = Path::new(&source_path).file_name().and_then(|s| s.to_str()).ok_or("Invalid file name")?;
    let staged = storage::uploads_dir().join(format!("{}_{}", uuid::Uuid::new_v4(), name));
    std::fs::copy(&source_path, &staged).map_err(|e| e.to_string())?;
    Ok(staged.to_string_lossy().to_string())
}

#[tauri::command]
fn create_spoof_job(app: AppHandle, state: State<AppState>, platform: String, input_path: String, config: SpoofConfig, base_name: Option<String>, output_dir: Option<String>) -> Result<JobRecord, String> {
    ensure_licensed()?;
    jobs::create_spoof_job(app, &state, platform, input_path, config, base_name, output_dir)
}

#[tauri::command]
fn create_image_spoof_job(app: AppHandle, state: State<AppState>, platform: String, input_path: String, config: ImageSpoofConfig, base_name: Option<String>, output_dir: Option<String>) -> Result<JobRecord, String> {
    ensure_licensed()?;
    jobs::create_image_spoof_job(app, &state, platform, input_path, config, base_name, output_dir)
}

#[tauri::command]
fn create_reddit_spoof_job(app: AppHandle, state: State<AppState>, platform: String, input_path: String, config: RedditSpoofConfig, base_name: Option<String>, output_dir: Option<String>) -> Result<JobRecord, String> {
    ensure_licensed()?;
    jobs::create_reddit_spoof_job(app, &state, platform, input_path, config, base_name, output_dir)
}

#[tauri::command]
fn create_processing_job(_platform: Option<String>, _input_path: Option<String>) -> Result<JobRecord, String> {
    Err("This action is not available. Use a platform-specific spoof instead.".into())
}

#[tauri::command]
fn cancel_spoof_job(state: State<AppState>, job_id: String) {
    state.cancel(&job_id);
}

#[tauri::command]
fn list_jobs(platform: String) -> Result<Vec<JobRecord>, String> {
    let conn = storage::open()?;
    storage::list_jobs(&conn, &platform)
}

#[tauri::command]
fn delete_job(job_id: String) -> Result<(), String> {
    let conn = storage::open()?;
    storage::delete_job(&conn, &job_id);
    Ok(())
}

#[tauri::command]
fn clear_jobs(platform: String) -> Result<(), String> {
    let conn = storage::open()?;
    storage::clear_jobs(&conn, &platform);
    Ok(())
}

#[tauri::command]
fn list_outputs(platform: String) -> Result<Vec<OutputRecord>, String> {
    let conn = storage::open()?;
    storage::list_outputs(&conn, &platform)
}

#[tauri::command]
fn delete_output(output_id: String) -> Result<(), String> {
    let conn = storage::open()?;
    if let Some(o) = storage::get_output(&conn, &output_id) {
        let _ = std::fs::remove_file(&o.file_path);
    }
    storage::delete_output(&conn, &output_id);
    Ok(())
}

#[tauri::command]
fn clear_outputs(platform: String) -> Result<(), String> {
    let conn = storage::open()?;
    storage::clear_outputs(&conn, &platform);
    Ok(())
}

#[tauri::command]
fn mark_outputs_reviewed(output_ids: Vec<String>) -> Result<(), String> {
    let conn = storage::open()?;
    storage::mark_reviewed(&conn, &output_ids);
    Ok(())
}

// ---------- concurrency / settings / paths ----------

#[tauri::command]
fn get_concurrency(state: State<AppState>) -> u32 {
    state.tasks.limit() as u32
}

#[tauri::command]
fn set_concurrency(state: State<AppState>, value: u32) -> Result<(), String> {
    let v = value.max(1);
    state.tasks.set_limit(v as usize);
    storage::save_concurrency(v)
}

#[tauri::command]
fn get_output_dir() -> Option<String> {
    storage::load_settings().output_dir
}

#[tauri::command]
fn set_output_dir(dir: Option<String>) -> Result<(), String> {
    storage::save_output_dir(dir)
}

#[tauri::command]
fn get_app_paths() -> AppPaths {
    AppPaths {
        uploads: storage::uploads_dir().to_string_lossy().to_string(),
        outputs: storage::outputs_dir().to_string_lossy().to_string(),
    }
}

#[tauri::command]
fn get_exif_devices() -> Vec<ExifDevice> {
    exif::device_list()
}

#[tauri::command]
fn open_outputs_folder(app: AppHandle, platform: Option<String>) -> Result<(), String> {
    // Outputs are written to <baseDir>/<batchFolder>, so open the folder that
    // actually holds the most recent output rather than outputs/<platform>.
    if let Some(p) = &platform {
        if let Ok(conn) = storage::open() {
            if let Ok(outs) = storage::list_outputs(&conn, p) {
                if let Some(first) = outs.iter().find(|o| Path::new(&o.file_path).exists()) {
                    if let Some(parent) = Path::new(&first.file_path).parent() {
                        return open_path(&app, &parent.to_string_lossy());
                    }
                }
            }
        }
    }
    // fallback: the base output directory (override → settings → default)
    let base = storage::load_settings()
        .output_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(storage::outputs_dir);
    let _ = std::fs::create_dir_all(&base);
    open_path(&app, &base.to_string_lossy())
}

#[tauri::command]
fn open_output_file(app: AppHandle, file_path: String) -> Result<(), String> {
    open_path(&app, &file_path)
}

// ---------- editor / video ----------

#[tauri::command]
fn probe_video_file(path: String) -> Result<ProbeResult, String> {
    ffmpeg::probe_video(&path)
}

#[tauri::command]
fn generate_auto_captions(app: AppHandle, video_path: String, language: Option<String>) -> Result<CaptionResult, String> {
    ensure_licensed()?;
    let emit = move |s: &str| { let _ = app.emit("caption-status", s.to_string()); };
    captions::generate(&video_path, language, &emit)
}

#[tauri::command]
fn cancel_caption_task() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn build_subtitle_track(caption: CaptionResult, config: SubtitleConfig, width: u32, height: u32) -> Result<String, String> {
    ensure_licensed()?;
    ass::build_ass(&caption, &config, width, height)
}

#[tauri::command]
fn create_video_job(app: AppHandle, state: State<AppState>, mode: String, input_path: String, template_id: Option<String>, render_options: Option<Value>) -> Result<VideoJobRecord, String> {
    ensure_licensed()?;
    let _ = template_id; // discarded (spec: templateId slot is dead)
    let m = mode.to_lowercase();
    if m != "quick" && m != "batch" {
        return Err("Unknown edit mode".into());
    }
    jobs::create_video_job(app, &state, m, input_path, render_options)
}

#[derive(Deserialize)]
struct CreateTemplateInput {
    name: String,
    #[serde(default)]
    subtitle_enabled: bool,
    #[serde(default)]
    subtitle_config: Option<SubtitleConfig>,
}

#[tauri::command]
fn create_video_template(input: CreateTemplateInput) -> Result<VideoTemplate, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("Template name required".into());
    }
    let conn = storage::open()?;
    storage::insert_template(&conn, name, input.subtitle_enabled, input.subtitle_config.as_ref())
}

#[tauri::command]
fn list_video_templates() -> Result<Vec<VideoTemplate>, String> {
    let conn = storage::open()?;
    storage::list_templates(&conn)
}

#[tauri::command]
fn delete_video_template(template_id: String) -> Result<(), String> {
    let conn = storage::open()?;
    storage::delete_template(&conn, &template_id);
    Ok(())
}

#[tauri::command]
fn list_video_jobs() -> Result<Vec<VideoJobRecord>, String> {
    let conn = storage::open()?;
    storage::list_video_jobs(&conn)
}

#[tauri::command]
fn cancel_video_job(state: State<AppState>, job_id: String) {
    state.cancel(&job_id);
}

#[tauri::command]
fn list_video_outputs() -> Result<Vec<VideoOutputRecord>, String> {
    let conn = storage::open()?;
    storage::list_video_outputs(&conn)
}

#[tauri::command]
fn delete_video_output(output_id: String) -> Result<(), String> {
    let conn = storage::open()?;
    storage::delete_video_output(&conn, &output_id);
    Ok(())
}

#[tauri::command]
fn open_video_outputs_folder(app: AppHandle) -> Result<(), String> {
    let dir = storage::video_outputs_dir();
    let _ = std::fs::create_dir_all(&dir);
    open_path(&app, &dir.to_string_lossy())
}

#[tauri::command]
fn open_video_output_file(app: AppHandle, file_path: String) -> Result<(), String> {
    open_path(&app, &file_path)
}

#[tauri::command]
fn create_video_batch() -> Result<Vec<VideoJobRecord>, String> {
    Err("No videos selected".into())
}

// ---------- converters ----------

#[tauri::command]
fn convert_image_file(input_path: String, target_format: String, quality: Option<u8>, base_name: Option<String>, output_dir: Option<String>) -> Result<String, String> {
    ensure_licensed()?;
    let base = base_name.unwrap_or_else(|| stem(&input_path));
    converters::convert_image(&input_path, &target_format, quality, &base, output_dir.as_deref())
}

#[tauri::command]
fn convert_video_file(input_path: String, target_format: String, base_name: Option<String>, output_dir: Option<String>) -> Result<String, String> {
    ensure_licensed()?;
    let base = base_name.unwrap_or_else(|| stem(&input_path));
    converters::convert_video(&input_path, &target_format, &base, output_dir.as_deref())
}

// ---------- license ----------

#[tauri::command]
fn get_license_status() -> LicenseStatus {
    license::status()
}

#[tauri::command]
fn activate_license_key(license_key: String) -> Result<(), String> {
    license::activate(&license_key)
}

#[tauri::command]
fn deactivate_license_key() -> Result<(), String> {
    license::deactivate()
}

#[tauri::command]
fn get_device_id() -> Result<String, String> {
    Ok(license::device_fingerprint())
}

// ---------- entrypoint ----------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    storage::ensure_dirs();
    let _ = storage::open();
    let concurrency = storage::load_settings().concurrency.unwrap_or(2) as usize;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new(concurrency))
        .setup(|app| {
            let _ = app.handle();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            stage_file_upload,
            create_spoof_job,
            create_image_spoof_job,
            create_reddit_spoof_job,
            create_processing_job,
            cancel_spoof_job,
            list_jobs,
            delete_job,
            clear_jobs,
            list_outputs,
            delete_output,
            clear_outputs,
            mark_outputs_reviewed,
            get_concurrency,
            set_concurrency,
            get_output_dir,
            set_output_dir,
            get_app_paths,
            get_exif_devices,
            open_outputs_folder,
            open_output_file,
            probe_video_file,
            generate_auto_captions,
            cancel_caption_task,
            build_subtitle_track,
            create_video_job,
            create_video_template,
            list_video_templates,
            delete_video_template,
            list_video_jobs,
            cancel_video_job,
            list_video_outputs,
            delete_video_output,
            open_video_outputs_folder,
            open_video_output_file,
            create_video_batch,
            convert_image_file,
            convert_video_file,
            get_license_status,
            activate_license_key,
            deactivate_license_key,
            get_device_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
