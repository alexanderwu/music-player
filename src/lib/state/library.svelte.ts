// Library state: the track list, scan progress, and search/sort.
//
// Search goes through SQL (search_tracks) rather than filtering in memory —
// the Phase 5 trade-off: in-memory filtering is simpler and instant at small
// scale, but the DB wins once libraries get big, and the DB is already the
// source of truth.

import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { getTracks, scanFolder, searchTracks } from "../ipc";
import type { ScanProgress, SortKey, Track } from "../types";

export const library = $state({
  tracks: [] as Track[],
  searchText: "",
  scanning: false,
  progress: null as ScanProgress | null,
  sortKey: "artist" as SortKey,
  sortAsc: true,
  loaded: false,
});

/** Tracks after client-side sorting (search already happened in SQL). */
export function sortedTracks(): Track[] {
  const { tracks, sortKey, sortAsc } = library;
  const dir = sortAsc ? 1 : -1;
  return [...tracks].sort((a, b) => {
    const av = a[sortKey];
    const bv = b[sortKey];
    if (av == null && bv == null) return 0;
    if (av == null) return dir;
    if (bv == null) return -dir;
    if (typeof av === "number" && typeof bv === "number")
      return (av - bv) * dir;
    return String(av).localeCompare(String(bv)) * dir;
  });
}

export function setSort(key: SortKey) {
  if (library.sortKey === key) {
    library.sortAsc = !library.sortAsc;
  } else {
    library.sortKey = key;
    library.sortAsc = true;
  }
}

export async function loadLibrary() {
  library.tracks = await getTracks();
  library.loaded = true;
}

let searchTimer: ReturnType<typeof setTimeout> | undefined;

export function setSearch(text: string) {
  library.searchText = text;
  clearTimeout(searchTimer);
  searchTimer = setTimeout(() => void refresh(), 150);
}

async function refresh() {
  const q = library.searchText.trim();
  library.tracks = q ? await searchTracks(q) : await getTracks();
}

/** Native folder picker → Rust scan. Progress arrives via events below. */
export async function addFolder() {
  const dir = await open({ directory: true, title: "Add music folder" });
  if (typeof dir !== "string") return;
  library.scanning = true;
  library.progress = null;
  try {
    await scanFolder(dir);
  } catch (e) {
    library.scanning = false;
    console.error("scan failed:", e);
  }
}

/** Call once at startup: subscribes to scanner events from Rust. */
export async function initLibraryListeners() {
  await listen<ScanProgress>("scan:progress", (event) => {
    library.scanning = true;
    library.progress = event.payload;
  });
  await listen<number>("scan:done", () => {
    library.scanning = false;
    library.progress = null;
    void refresh();
  });
}
