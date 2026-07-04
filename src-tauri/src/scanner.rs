//! Folder scanning and tag reading — the clearest "this belongs in Rust"
//! work in the app: a tight native loop over thousands of files, run on a
//! background thread, streaming progress to the UI via events.

use std::path::{Path, PathBuf};
use std::time::Instant;

use lofty::picture::{MimeType, Picture};
use lofty::prelude::*;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use walkdir::WalkDir;

use crate::db::{self, Db};
use crate::models::{DropResult, ScanProgress, Track};

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "aac", "flac", "ogg", "oga", "opus", "wav", "aiff", "aif",
];

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

struct TagData {
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_secs: Option<f64>,
    track_no: Option<u32>,
    cover_path: Option<String>,
}

fn covers_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Write embedded cover art to the on-disk cache, named by content hash so
/// every track of an album shares one file. Disk beats base64-over-IPC here:
/// art repeats per album, is big, and cached files let the webview cache too.
fn cache_cover(picture: &Picture, covers: &Path) -> Option<String> {
    let ext = match picture.mime_type() {
        Some(MimeType::Png) => "png",
        _ => "jpg",
    };
    let hash = hex::encode(Sha256::digest(picture.data()));
    let file = covers.join(format!("{hash}.{ext}"));
    if !file.exists() {
        std::fs::write(&file, picture.data()).ok()?;
    }
    Some(file.to_string_lossy().into_owned())
}

/// Read tags with lofty, falling back to the filename when tags are missing
/// or unreadable — real libraries are dirty.
fn read_tags(path: &Path, covers: &Path) -> TagData {
    let fallback_title = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let tagged = match lofty::read_from_path(path) {
        Ok(tagged) => tagged,
        Err(_) => {
            return TagData {
                title: fallback_title,
                artist: None,
                album: None,
                duration_secs: None,
                track_no: None,
                cover_path: None,
            }
        }
    };

    let duration_secs = Some(tagged.properties().duration().as_secs_f64());
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let non_empty = |s: String| if s.trim().is_empty() { None } else { Some(s) };

    TagData {
        title: tag
            .and_then(|t| t.title().map(|s| s.into_owned()))
            .and_then(non_empty)
            .unwrap_or(fallback_title),
        artist: tag
            .and_then(|t| t.artist().map(|s| s.into_owned()))
            .and_then(non_empty),
        album: tag
            .and_then(|t| t.album().map(|s| s.into_owned()))
            .and_then(non_empty),
        duration_secs,
        track_no: tag.and_then(|t| t.track()),
        cover_path: tag
            .and_then(|t| t.pictures().first())
            .and_then(|picture| cache_cover(picture, covers)),
    }
}

fn upsert_tag_data(app: &AppHandle, path: &Path, tags: &TagData) -> Result<Track, String> {
    let db = app.state::<Db>();
    let conn = db.0.lock().unwrap();
    db::upsert_track(
        &conn,
        &path.to_string_lossy(),
        &tags.title,
        tags.artist.as_deref(),
        tags.album.as_deref(),
        tags.duration_secs,
        tags.track_no,
        tags.cover_path.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// The synchronous scan, meant to run on a blocking thread. Emits
/// `scan:progress` (throttled — the UI can't paint faster anyway) and a
/// final `scan:done`.
fn scan_folder_blocking(app: &AppHandle, folder: &str) -> Result<usize, String> {
    // Let the webview load audio from this tree via asset:// — this tree
    // only, not the whole disk.
    app.asset_protocol_scope()
        .allow_directory(folder, true)
        .map_err(|e| e.to_string())?;

    let covers = covers_dir(app)?;

    {
        let db = app.state::<Db>();
        let conn = db.0.lock().unwrap();
        db::remember_folder(&conn, folder).map_err(|e| e.to_string())?;
    }

    let files: Vec<PathBuf> = WalkDir::new(folder)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_audio(path))
        .collect();

    let total = files.len();
    let mut last_emit = Instant::now();

    for (i, file) in files.iter().enumerate() {
        let tags = read_tags(file, &covers);
        upsert_tag_data(app, file, &tags)?;

        if last_emit.elapsed().as_millis() >= 30 || i + 1 == total {
            let _ = app.emit(
                "scan:progress",
                ScanProgress {
                    done: i + 1,
                    total,
                    current_file: file
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                },
            );
            last_emit = Instant::now();
        }
    }

    let _ = app.emit("scan:done", total);
    Ok(total)
}

/// Async command wrapper: the heavy loop runs via `spawn_blocking` so it
/// never occupies the async runtime's core threads — UI thread sacred,
/// work elsewhere, progress via events.
#[tauri::command]
pub async fn scan_folder(app: AppHandle, path: String) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || scan_folder_blocking(&app, &path))
        .await
        .map_err(|e| e.to_string())?
}

/// Dropped paths from the native drag & drop event: audio files are added
/// to the library and returned for queueing; folders kick off full scans.
#[tauri::command]
pub async fn handle_dropped_paths(
    app: AppHandle,
    paths: Vec<String>,
) -> Result<DropResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let covers = covers_dir(&app)?;
        let mut queued = Vec::new();
        let mut folders_scanned = 0;

        for raw in paths {
            let path = PathBuf::from(&raw);
            if path.is_dir() {
                scan_folder_blocking(&app, &raw)?;
                folders_scanned += 1;
            } else if is_audio(&path) {
                app.asset_protocol_scope()
                    .allow_file(&path)
                    .map_err(|e| e.to_string())?;
                let tags = read_tags(&path, &covers);
                queued.push(upsert_tag_data(&app, &path, &tags)?);
            }
        }

        Ok(DropResult {
            queued,
            folders_scanned,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
