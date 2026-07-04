//! The database is owned by the Rust core; the frontend only ever sees the
//! typed commands below — the same layering as a web app's API server.
//!
//! The connection lives in managed state as `Mutex<Connection>`: commands can
//! run concurrently, and a SQLite connection isn't safe to share unsynchronized.
//! One serialized connection is plenty for this app (a pool would be the
//! upgrade for write-heavy workloads).

use std::sync::Mutex;

use rusqlite::{params, Connection};
use tauri::{AppHandle, Manager, State};

use crate::models::{Playlist, Track};

pub struct Db(pub Mutex<Connection>);

pub fn init(app: &AppHandle) -> Result<Db, Box<dyn std::error::Error>> {
    // Ask Tauri for the per-OS data dir — never hardcode it.
    // (~/.local/share on Linux, ~/Library/Application Support on macOS,
    // %APPDATA% on Windows.)
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let conn = Connection::open(dir.join("library.db"))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(Db(Mutex::new(conn)))
}

/// Trivial `user_version`-based migrations: each block below runs at most
/// once per database. Version 1 is the initial schema.
fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 1 {
        conn.execute_batch(include_str!("../schema.sql"))?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    Ok(())
}

fn track_from_row(row: &rusqlite::Row) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album: row.get(4)?,
        duration_secs: row.get(5)?,
        track_no: row.get(6)?,
        cover_path: row.get(7)?,
    })
}

const TRACK_COLUMNS: &str = "id, path, title, artist, album, duration_secs, track_no, cover_path";

// ---- Helpers used by the scanner (not exposed over IPC) ----

#[allow(clippy::too_many_arguments)] // mirrors the tracks table's columns
pub fn upsert_track(
    conn: &Connection,
    path: &str,
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    duration_secs: Option<f64>,
    track_no: Option<u32>,
    cover_path: Option<&str>,
) -> rusqlite::Result<Track> {
    conn.query_row(
        &format!(
            "INSERT INTO tracks (path, title, artist, album, duration_secs, track_no, cover_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (path) DO UPDATE SET
               title = excluded.title,
               artist = excluded.artist,
               album = excluded.album,
               duration_secs = excluded.duration_secs,
               track_no = excluded.track_no,
               cover_path = excluded.cover_path
             RETURNING {TRACK_COLUMNS}"
        ),
        params![path, title, artist, album, duration_secs, track_no, cover_path],
        track_from_row,
    )
}

pub fn remember_folder(conn: &Connection, path: &str) -> rusqlite::Result<()> {
    conn.execute("INSERT OR IGNORE INTO folders (path) VALUES (?1)", [path])?;
    Ok(())
}

/// Folders scanned in previous sessions — re-allowed on the asset-protocol
/// scope at startup so the webview can keep playing the library.
pub fn folder_paths(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM folders ORDER BY path")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    rows.collect()
}

// ---- Library commands ----

#[tauri::command]
pub fn get_tracks(db: State<Db>) -> Result<Vec<Track>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks
             ORDER BY artist COLLATE NOCASE, album COLLATE NOCASE, track_no, title COLLATE NOCASE"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], track_from_row)
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<_>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_tracks(db: State<Db>, query: String) -> Result<Vec<Track>, String> {
    // Escape LIKE wildcards so searching for "100%" behaves literally.
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {TRACK_COLUMNS} FROM tracks
             WHERE title LIKE ?1 ESCAPE '\\'
                OR artist LIKE ?1 ESCAPE '\\'
                OR album LIKE ?1 ESCAPE '\\'
             ORDER BY artist COLLATE NOCASE, album COLLATE NOCASE, track_no, title COLLATE NOCASE"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([&pattern], track_from_row)
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<_>>().map_err(|e| e.to_string())
}

// ---- Playlist commands ----

#[tauri::command]
pub fn get_playlists(db: State<Db>) -> Result<Vec<Playlist>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT p.id, p.name, COUNT(pt.track_id)
             FROM playlists p
             LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
             GROUP BY p.id, p.name
             ORDER BY p.name COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Playlist {
                id: row.get(0)?,
                name: row.get(1)?,
                track_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<_>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_playlist(db: State<Db>, name: String) -> Result<Playlist, String> {
    let conn = db.0.lock().unwrap();
    conn.execute("INSERT INTO playlists (name) VALUES (?1)", [&name])
        .map_err(|e| e.to_string())?;
    Ok(Playlist {
        id: conn.last_insert_rowid(),
        name,
        track_count: 0,
    })
}

#[tauri::command]
pub fn rename_playlist(db: State<Db>, id: i64, name: String) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    conn.execute("UPDATE playlists SET name = ?1 WHERE id = ?2", params![name, id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_playlist(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM playlists WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_playlist_tracks(db: State<Db>, id: i64) -> Result<Vec<Track>, String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT t.{cols} FROM playlist_tracks pt
             JOIN tracks t ON t.id = pt.track_id
             WHERE pt.playlist_id = ?1
             ORDER BY pt.position",
            cols = TRACK_COLUMNS.replace(", ", ", t."),
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id], track_from_row)
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<_>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_to_playlist(
    db: State<Db>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<(), String> {
    let mut conn = db.0.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    {
        let next: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(position) + 1, 0) FROM playlist_tracks WHERE playlist_id = ?1",
                [playlist_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut stmt = tx
            .prepare(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
            )
            .map_err(|e| e.to_string())?;
        for (offset, track_id) in track_ids.iter().enumerate() {
            stmt.execute(params![playlist_id, track_id, next + offset as i64])
                .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())
}

/// Replace a playlist's contents with `track_ids` in order. Used both for
/// drag-to-reorder and as the "rewrite everything atomically" primitive —
/// simpler and more robust than shuffling position values in place.
fn write_playlist_order(
    conn: &mut Connection,
    playlist_id: i64,
    track_ids: &[i64],
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    {
        tx.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1",
            [playlist_id],
        )?;
        let mut stmt = tx.prepare(
            "INSERT INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
        )?;
        for (position, track_id) in track_ids.iter().enumerate() {
            stmt.execute(params![playlist_id, track_id, position as i64])?;
        }
    }
    tx.commit()
}

#[tauri::command]
pub fn set_playlist_order(
    db: State<Db>,
    playlist_id: i64,
    track_ids: Vec<i64>,
) -> Result<(), String> {
    let mut conn = db.0.lock().unwrap();
    write_playlist_order(&mut conn, playlist_id, &track_ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_from_playlist(
    db: State<Db>,
    playlist_id: i64,
    position: i64,
) -> Result<(), String> {
    let mut conn = db.0.lock().unwrap();
    let remaining: Vec<i64> = {
        let mut stmt = conn
            .prepare(
                "SELECT track_id FROM playlist_tracks
                 WHERE playlist_id = ?1 AND position <> ?2
                 ORDER BY position",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![playlist_id, position], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<_>>().map_err(|e| e.to_string())?
    };
    write_playlist_order(&mut conn, playlist_id, &remaining).map_err(|e| e.to_string())
}
