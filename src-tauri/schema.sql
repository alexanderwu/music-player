-- Schema v1. Applied when PRAGMA user_version < 1 (see db.rs).

CREATE TABLE IF NOT EXISTS folders (
    id   INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS tracks (
    id            INTEGER PRIMARY KEY,
    path          TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    artist        TEXT,
    album         TEXT,
    duration_secs REAL,
    track_no      INTEGER,
    cover_path    TEXT
);

CREATE INDEX IF NOT EXISTS idx_tracks_title  ON tracks (title);
CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks (artist);
CREATE INDEX IF NOT EXISTS idx_tracks_album  ON tracks (album);

CREATE TABLE IF NOT EXISTS playlists (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

-- Ordered membership: `position` is a dense 0-based index per playlist.
CREATE TABLE IF NOT EXISTS playlist_tracks (
    playlist_id INTEGER NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
    track_id    INTEGER NOT NULL REFERENCES tracks (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, position)
);
