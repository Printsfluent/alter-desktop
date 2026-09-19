// SQLite persistence + settings.json + app data folders (spec §5/§9 storage).
use crate::models::*;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ----- paths -----

pub fn data_root() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::env::temp_dir());
    base.join("Alter")
}
pub fn db_path() -> PathBuf { data_root().join("alter.db") }
pub fn settings_path() -> PathBuf { data_root().join("settings.json") }
pub fn uploads_dir() -> PathBuf { data_root().join("uploads") }
pub fn outputs_dir() -> PathBuf { data_root().join("outputs") }
pub fn video_outputs_dir() -> PathBuf { data_root().join("video-outputs") }
pub fn captions_work_dir() -> PathBuf { data_root().join("captions-work") }

pub fn ensure_dirs() {
    for d in [data_root(), uploads_dir(), outputs_dir(), video_outputs_dir()] {
        let _ = std::fs::create_dir_all(d);
    }
}

// ----- connection + schema -----

pub fn open() -> Result<Connection, String> {
    ensure_dirs();
    let conn = Connection::open(db_path()).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;\nPRAGMA busy_timeout=5000;\nPRAGMA synchronous=NORMAL;",
    ).map_err(|e| e.to_string())?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY, platform TEXT, input_path TEXT, output_path TEXT,
            status TEXT, progress INTEGER, error TEXT, created_at TEXT, updated_at TEXT
        );
        CREATE TABLE IF NOT EXISTS outputs (
            id TEXT PRIMARY KEY, job_id TEXT, platform TEXT, file_name TEXT, file_path TEXT,
            created_at TEXT, reviewed INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS video_templates (
            id TEXT PRIMARY KEY, name TEXT, subtitle_enabled INTEGER, caption_text TEXT,
            style_json TEXT NOT NULL, created_at TEXT, updated_at TEXT
        );
        CREATE TABLE IF NOT EXISTS video_jobs (
            id TEXT PRIMARY KEY, mode TEXT, template_id TEXT, template_name TEXT,
            input_path TEXT, output_path TEXT, status TEXT, progress INTEGER, error TEXT,
            render_options TEXT, created_at TEXT, updated_at TEXT
        );
        CREATE TABLE IF NOT EXISTS video_outputs (
            id TEXT PRIMARY KEY, job_id TEXT, template_name TEXT, file_name TEXT,
            file_path TEXT, created_at TEXT
        );
        "#,
    ).map_err(|e| e.to_string())?;
    // idempotent migrations (errors swallowed — duplicate-column etc.)
    let _ = conn.execute("ALTER TABLE video_jobs ADD COLUMN render_options TEXT", []);
    let _ = conn.execute("ALTER TABLE video_templates ADD COLUMN subtitle_config TEXT", []);
    let _ = conn.execute("ALTER TABLE outputs ADD COLUMN reviewed INTEGER NOT NULL DEFAULT 0", []);
    Ok(())
}

// ----- settings.json -----

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<u32>,
}

pub fn load_settings() -> Settings {
    match std::fs::read_to_string(settings_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(s: &Settings) -> Result<(), String> {
    ensure_dirs();
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(), json).map_err(|e| e.to_string())
}

pub fn save_output_dir(dir: Option<String>) -> Result<(), String> {
    let mut s = load_settings();
    s.output_dir = dir;
    save_settings(&s)
}
pub fn save_concurrency(value: u32) -> Result<(), String> {
    let mut s = load_settings();
    s.concurrency = Some(value.max(1));
    save_settings(&s)
}

// ----- jobs -----

pub fn insert_job(conn: &Connection, platform: &str, input_path: &str) -> Result<JobRecord, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = now();
    conn.execute(
        "INSERT INTO jobs (id,platform,input_path,output_path,status,progress,error,created_at,updated_at)
         VALUES (?1,?2,?3,NULL,'queued',0,NULL,?4,?4)",
        params![id, platform, input_path, ts],
    ).map_err(|e| e.to_string())?;
    Ok(JobRecord {
        id, platform: platform.into(), input_path: input_path.into(), output_path: None,
        status: "queued".into(), progress: 0, error: None, created_at: ts.clone(), updated_at: ts,
    })
}

pub fn update_job(conn: &Connection, id: &str, status: &str, progress: i64, output_path: Option<&str>, error: Option<&str>) {
    let _ = conn.execute(
        "UPDATE jobs SET status=?2, progress=?3, output_path=COALESCE(?4,output_path), error=?5, updated_at=?6 WHERE id=?1",
        params![id, status, progress, output_path, error, now()],
    );
}

fn map_job(row: &rusqlite::Row) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        id: row.get(0)?, platform: row.get(1)?, input_path: row.get(2)?, output_path: row.get(3)?,
        status: row.get(4)?, progress: row.get(5)?, error: row.get(6)?,
        created_at: row.get(7)?, updated_at: row.get(8)?,
    })
}

pub fn list_jobs(conn: &Connection, platform: &str) -> Result<Vec<JobRecord>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,platform,input_path,output_path,status,progress,error,created_at,updated_at
         FROM jobs WHERE platform=?1 ORDER BY created_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![platform], map_job).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_job(conn: &Connection, id: &str) -> Option<JobRecord> {
    conn.query_row(
        "SELECT id,platform,input_path,output_path,status,progress,error,created_at,updated_at FROM jobs WHERE id=?1",
        params![id], map_job,
    ).ok()
}

pub fn delete_job(conn: &Connection, id: &str) {
    let _ = conn.execute("DELETE FROM jobs WHERE id=?1", params![id]);
}
pub fn clear_jobs(conn: &Connection, platform: &str) {
    let _ = conn.execute(
        "DELETE FROM jobs WHERE status IN ('completed','failed') AND platform=?1",
        params![platform],
    );
}

// ----- outputs -----

pub fn insert_output(conn: &Connection, job_id: &str, platform: &str, file_name: &str, file_path: &str) -> Result<OutputRecord, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = now();
    conn.execute(
        "INSERT INTO outputs (id,job_id,platform,file_name,file_path,created_at,reviewed)
         VALUES (?1,?2,?3,?4,?5,?6,0)",
        params![id, job_id, platform, file_name, file_path, ts],
    ).map_err(|e| e.to_string())?;
    Ok(OutputRecord {
        id, job_id: job_id.into(), platform: platform.into(), file_name: file_name.into(),
        file_path: file_path.into(), created_at: ts, reviewed: 0,
    })
}

fn map_output(row: &rusqlite::Row) -> rusqlite::Result<OutputRecord> {
    Ok(OutputRecord {
        id: row.get(0)?, job_id: row.get(1)?, platform: row.get(2)?, file_name: row.get(3)?,
        file_path: row.get(4)?, created_at: row.get(5)?, reviewed: row.get(6)?,
    })
}

pub fn list_outputs(conn: &Connection, platform: &str) -> Result<Vec<OutputRecord>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,job_id,platform,file_name,file_path,created_at,reviewed
         FROM outputs WHERE platform=?1 ORDER BY created_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![platform], map_output).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_output(conn: &Connection, id: &str) -> Option<OutputRecord> {
    conn.query_row(
        "SELECT id,job_id,platform,file_name,file_path,created_at,reviewed FROM outputs WHERE id=?1",
        params![id], map_output,
    ).ok()
}

pub fn delete_output(conn: &Connection, id: &str) {
    let _ = conn.execute("DELETE FROM outputs WHERE id=?1", params![id]);
}
pub fn clear_outputs(conn: &Connection, platform: &str) {
    let _ = conn.execute("DELETE FROM outputs WHERE platform=?1", params![platform]);
}
pub fn mark_reviewed(conn: &Connection, ids: &[String]) {
    for id in ids {
        let _ = conn.execute("UPDATE outputs SET reviewed = 1 WHERE id = ?1", params![id]);
    }
}

// ----- video templates -----

pub fn insert_template(conn: &Connection, name: &str, subtitle_enabled: bool, subtitle_config: Option<&SubtitleConfig>) -> Result<VideoTemplate, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = now();
    let sc_json = subtitle_config.map(|c| serde_json::to_string(c).unwrap_or_default());
    conn.execute(
        "INSERT INTO video_templates (id,name,subtitle_enabled,caption_text,style_json,created_at,updated_at,subtitle_config)
         VALUES (?1,?2,?3,NULL,'{}',?4,?4,?5)",
        params![id, name, subtitle_enabled as i64, ts, sc_json],
    ).map_err(|e| e.to_string())?;
    Ok(VideoTemplate {
        id, name: name.into(), subtitle_enabled, caption_text: None,
        subtitle_config: subtitle_config.cloned(), created_at: ts.clone(), updated_at: ts,
    })
}

pub fn list_templates(conn: &Connection) -> Result<Vec<VideoTemplate>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,name,subtitle_enabled,caption_text,created_at,updated_at,subtitle_config
         FROM video_templates ORDER BY updated_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let sc: Option<String> = row.get(6)?;
        Ok(VideoTemplate {
            id: row.get(0)?, name: row.get(1)?,
            subtitle_enabled: row.get::<_, i64>(2)? != 0,
            caption_text: row.get(3)?, created_at: row.get(4)?, updated_at: row.get(5)?,
            subtitle_config: sc.and_then(|s| serde_json::from_str(&s).ok()),
        })
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_template(conn: &Connection, id: &str) -> Option<VideoTemplate> {
    let mut stmt = conn.prepare(
        "SELECT id,name,subtitle_enabled,caption_text,created_at,updated_at,subtitle_config
         FROM video_templates WHERE id=?1",
    ).ok()?;
    stmt.query_row(params![id], |row| {
        let sc: Option<String> = row.get(6)?;
        Ok(VideoTemplate {
            id: row.get(0)?, name: row.get(1)?, subtitle_enabled: row.get::<_, i64>(2)? != 0,
            caption_text: row.get(3)?, created_at: row.get(4)?, updated_at: row.get(5)?,
            subtitle_config: sc.and_then(|s| serde_json::from_str(&s).ok()),
        })
    }).ok()
}

pub fn delete_template(conn: &Connection, id: &str) {
    let _ = conn.execute("DELETE FROM video_templates WHERE id=?1", params![id]);
}

// ----- video jobs / outputs -----

pub fn insert_video_job(conn: &Connection, mode: &str, input_path: &str, template_id: Option<&str>, template_name: Option<&str>, render_options: Option<&str>) -> Result<VideoJobRecord, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = now();
    conn.execute(
        "INSERT INTO video_jobs (id,mode,template_id,template_name,input_path,output_path,status,progress,error,render_options,created_at,updated_at)
         VALUES (?1,?2,?3,?4,?5,NULL,'queued',0,NULL,?6,?7,?7)",
        params![id, mode, template_id, template_name, input_path, render_options, ts],
    ).map_err(|e| e.to_string())?;
    Ok(VideoJobRecord {
        id, mode: mode.into(), template_id: template_id.map(|s| s.into()),
        template_name: template_name.map(|s| s.into()), input_path: input_path.into(),
        output_path: None, status: "queued".into(), progress: 0, error: None,
        render_options: render_options.map(|s| s.into()), created_at: ts.clone(), updated_at: ts,
    })
}

pub fn update_video_job(conn: &Connection, id: &str, status: &str, progress: i64, output_path: Option<&str>, error: Option<&str>) {
    let _ = conn.execute(
        "UPDATE video_jobs SET status=?2, progress=?3, output_path=COALESCE(?4,output_path), error=?5, updated_at=?6 WHERE id=?1",
        params![id, status, progress, output_path, error, now()],
    );
}

fn map_video_job(row: &rusqlite::Row) -> rusqlite::Result<VideoJobRecord> {
    Ok(VideoJobRecord {
        id: row.get(0)?, mode: row.get(1)?, template_id: row.get(2)?, template_name: row.get(3)?,
        input_path: row.get(4)?, output_path: row.get(5)?, status: row.get(6)?, progress: row.get(7)?,
        error: row.get(8)?, render_options: row.get(9)?, created_at: row.get(10)?, updated_at: row.get(11)?,
    })
}

pub fn get_video_job(conn: &Connection, id: &str) -> Option<VideoJobRecord> {
    conn.query_row(
        "SELECT id,mode,template_id,template_name,input_path,output_path,status,progress,error,render_options,created_at,updated_at
         FROM video_jobs WHERE id=?1",
        params![id], map_video_job,
    ).ok()
}

pub fn list_video_jobs(conn: &Connection) -> Result<Vec<VideoJobRecord>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,mode,template_id,template_name,input_path,output_path,status,progress,error,render_options,created_at,updated_at
         FROM video_jobs ORDER BY created_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], map_video_job).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn insert_video_output(conn: &Connection, job_id: &str, template_name: Option<&str>, file_name: &str, file_path: &str) -> Result<VideoOutputRecord, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = now();
    conn.execute(
        "INSERT INTO video_outputs (id,job_id,template_name,file_name,file_path,created_at)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, job_id, template_name, file_name, file_path, ts],
    ).map_err(|e| e.to_string())?;
    Ok(VideoOutputRecord {
        id, job_id: job_id.into(), template_name: template_name.map(|s| s.into()),
        file_name: file_name.into(), file_path: file_path.into(), created_at: ts,
    })
}

pub fn list_video_outputs(conn: &Connection) -> Result<Vec<VideoOutputRecord>, String> {
    let mut stmt = conn.prepare(
        "SELECT id,job_id,template_name,file_name,file_path,created_at FROM video_outputs ORDER BY created_at DESC",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| Ok(VideoOutputRecord {
        id: row.get(0)?, job_id: row.get(1)?, template_name: row.get(2)?, file_name: row.get(3)?,
        file_path: row.get(4)?, created_at: row.get(5)?,
    })).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_video_output(conn: &Connection, id: &str) {
    let _ = conn.execute("DELETE FROM video_outputs WHERE id=?1", params![id]);
}

// unique output path helper (reddit_spoof::unique_output_path — counter starts at 2)
pub fn unique_output_path(base: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = base.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    let mut n = 2;
    loop {
        let cand = base.join(format!("{stem}-{n}.{ext}"));
        if !cand.exists() {
            return cand;
        }
        n += 1;
    }
}
