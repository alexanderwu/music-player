// These types mirror the Rust structs in src-tauri/src/models.rs exactly.
// Rust uses snake_case fields; #[serde(rename_all = "camelCase")] on the
// Rust side makes the wire format match what you see here.

export interface Track {
  id: number;
  path: string;
  title: string;
  artist: string | null;
  album: string | null;
  durationSecs: number | null;
  trackNo: number | null;
  coverPath: string | null;
}

export interface Playlist {
  id: number;
  name: string;
  trackCount: number;
}

export interface ScanProgress {
  done: number;
  total: number;
  currentFile: string;
}

/** Result of dropping files/folders onto the window. */
export interface DropResult {
  /** Audio files that were dropped directly — ready to queue. */
  queued: Track[];
  /** Number of dropped folders that kicked off a scan. */
  foldersScanned: number;
}

export type RepeatMode = "off" | "all" | "one";

export type SortKey = "title" | "artist" | "album" | "durationSecs" | "trackNo";
