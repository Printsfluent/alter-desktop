// Background job orchestration: slot acquisition, DB updates, event emission.
use crate::models::*;
use crate::state::{AppState, CancelRegistry};
use crate::state::TaskManager;
use crate::{editor, reddit_spoof, spoof_image, spoof_video, storage};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Emitter};

fn stem(input: &str) -> String {
    Path::new(input).file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string()
}

fn emit_job(app: &AppHandle, conn: &rusqlite::Connection, id: &str) {
    if let Some(rec) = storage::get_job(conn, id) {
        let _ = app.emit("job-progress", &rec);
    }
}

// Generic spoof-job scaffold. `run` produces the outputs via callbacks.
fn spawn_spoof<F>(
    app: AppHandle,
    tasks: Arc<TaskManager>,
    reg: CancelRegistry,
    job_id: String,
    platform: String,
    cancel: Arc<AtomicBool>,
    run: F,
) where
    F: FnOnce(&Arc<AtomicBool>, &mut dyn FnMut(i64), &mut dyn FnMut(&Path)) -> Result<(), String> + Send + 'static,
{
    thread::spawn(move || {
        tasks.acquire();
        if let Ok(conn) = storage::open() {
            storage::update_job(&conn, &job_id, "processing", 0, None, None);
            emit_job(&app, &conn, &job_id);

            let res = {
                let mut on_prog = |pct: i64| {
                    storage::update_job(&conn, &job_id, "processing", pct, None, None);
                    emit_job(&app, &conn, &job_id);
                };
                let mut on_out = |p: &Path| {
                    let fname = p.file_name().and_then(|s| s.to_str()).unwrap_or("output").to_string();
                    if let Ok(rec) = storage::insert_output(&conn, &job_id, &platform, &fname, &p.to_string_lossy()) {
                        let _ = app.emit("job-completed", &rec);
                    }
                };
                run(&cancel, &mut on_prog, &mut on_out)
            };

            match res {
                Ok(()) => storage::update_job(&conn, &job_id, "completed", 100, None, None),
                Err(e) => {
                    let prog = storage::get_job(&conn, &job_id).map(|j| j.progress).unwrap_or(0);
                    storage::update_job(&conn, &job_id, "failed", prog, None, Some(&e));
                }
            }
            emit_job(&app, &conn, &job_id);
        }
        tasks.release();
        reg.lock().unwrap().remove(&job_id);
    });
}

fn out_dir_for(platform: &str, output_dir: Option<String>) -> PathBuf {
    match output_dir {
        Some(d) => PathBuf::from(d),
        None => storage::outputs_dir().join(platform),
    }
}

pub fn create_spoof_job(app: AppHandle, state: &AppState, platform: String, input: String, cfg: SpoofConfig, base_name: Option<String>, output_dir: Option<String>) -> Result<JobRecord, String> {
    let conn = storage::open()?;
    let job = storage::insert_job(&conn, &platform, &input)?;
    drop(conn);
    let out_dir = out_dir_for(&platform, output_dir);
    let base = base_name.unwrap_or_else(|| stem(&input));
    let cancel = state.register_cancel(&job.id);
    spawn_spoof(app, state.tasks.clone(), state.registry(), job.id.clone(), platform, cancel, move |c, p, o| {
        spoof_video::render(&input, &cfg, &out_dir, &base, c, p, o)
    });
    Ok(job)
}

pub fn create_image_spoof_job(app: AppHandle, state: &AppState, platform: String, input: String, cfg: ImageSpoofConfig, base_name: Option<String>, output_dir: Option<String>) -> Result<JobRecord, String> {
    let conn = storage::open()?;
    let job = storage::insert_job(&conn, &platform, &input)?;
    drop(conn);
    let out_dir = out_dir_for(&platform, output_dir);
    let base = base_name.unwrap_or_else(|| stem(&input));
    let cancel = state.register_cancel(&job.id);
    spawn_spoof(app, state.tasks.clone(), state.registry(), job.id.clone(), platform, cancel, move |c, p, o| {
        spoof_image::render(&input, &cfg, &out_dir, &base, c, p, o)
    });
    Ok(job)
}

pub fn create_reddit_spoof_job(app: AppHandle, state: &AppState, platform: String, input: String, cfg: RedditSpoofConfig, base_name: Option<String>, output_dir: Option<String>) -> Result<JobRecord, String> {
    let conn = storage::open()?;
    let job = storage::insert_job(&conn, &platform, &input)?;
    drop(conn);
    let out_dir = out_dir_for(&platform, output_dir);
    let base = base_name.unwrap_or_else(|| stem(&input));
    let cancel = state.register_cancel(&job.id);
    spawn_spoof(app, state.tasks.clone(), state.registry(), job.id.clone(), platform, cancel, move |c, p, o| {
        reddit_spoof::render(&input, &cfg, &out_dir, &base, c, p, o)
    });
    Ok(job)
}

// ----- video jobs (editor) -----

pub fn create_video_job(app: AppHandle, state: &AppState, mode: String, input: String, render_options: Option<Value>) -> Result<VideoJobRecord, String> {
    let ro_str = render_options.as_ref().map(|v| v.to_string());
    let conn = storage::open()?;
    let job = storage::insert_video_job(&conn, &mode, &input, None, None, ro_str.as_deref())?;
    drop(conn);
    let job_id = job.id.clone();
    let base = stem(&input);
    let out_dir = storage::video_outputs_dir();
    let cancel = state.register_cancel(&job_id);
    let tasks = state.tasks.clone();
    let reg = state.registry();
    let ro = render_options.unwrap_or(Value::Null);

    thread::spawn(move || {
        tasks.acquire();
        if let Ok(conn) = storage::open() {
            storage::update_video_job(&conn, &job_id, "processing", 0, None, None);
            emit_video(&app, &conn, &job_id);
            let mut on_prog = |pct: i64| {
                storage::update_video_job(&conn, &job_id, "processing", pct, None, None);
                emit_video(&app, &conn, &job_id);
            };
            let res = editor::render_video(&input, &ro, &out_dir, &base, &cancel, &mut on_prog);
            match res {
                Ok(path) => {
                    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("video.mp4").to_string();
                    let _ = storage::insert_video_output(&conn, &job_id, None, &fname, &path.to_string_lossy());
                    storage::update_video_job(&conn, &job_id, "completed", 100, Some(&path.to_string_lossy()), None);
                    if let Some(rec) = storage::get_video_job(&conn, &job_id) {
                        let _ = app.emit("video-job-completed", &rec);
                    }
                }
                Err(e) => storage::update_video_job(&conn, &job_id, "failed", 0, None, Some(&e)),
            }
            emit_video(&app, &conn, &job_id);
        }
        tasks.release();
        reg.lock().unwrap().remove(&job_id);
    });
    Ok(job)
}

fn emit_video(app: &AppHandle, conn: &rusqlite::Connection, id: &str) {
    if let Some(rec) = storage::get_video_job(conn, id) {
        let _ = app.emit("video-job-progress", &rec);
    }
}
