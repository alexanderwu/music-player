// Typed wrappers around the IPC boundary. `invoke()` itself returns
// Promise<unknown> — TypeScript can't see Rust — so this module is the one
// place where we assert what the Rust commands actually return. Keep it in
// sync with src-tauri/src/{db,scanner}.rs.

import { invoke } from "@tauri-apps/api/core";
import type { DropResult, Playlist, Track } from "./types";

// ---- Library / scanning ----

export function scanFolder(path: string): Promise<number> {
  return invoke<number>("scan_folder", { path });
}

export function getTracks(): Promise<Track[]> {
  return invoke<Track[]>("get_tracks");
}

export function searchTracks(query: string): Promise<Track[]> {
  return invoke<Track[]>("search_tracks", { query });
}

export function handleDroppedPaths(paths: string[]): Promise<DropResult> {
  return invoke<DropResult>("handle_dropped_paths", { paths });
}

// ---- Playlists ----

export function getPlaylists(): Promise<Playlist[]> {
  return invoke<Playlist[]>("get_playlists");
}

export function createPlaylist(name: string): Promise<Playlist> {
  return invoke<Playlist>("create_playlist", { name });
}

export function renamePlaylist(id: number, name: string): Promise<void> {
  return invoke("rename_playlist", { id, name });
}

export function deletePlaylist(id: number): Promise<void> {
  return invoke("delete_playlist", { id });
}

export function getPlaylistTracks(id: number): Promise<Track[]> {
  return invoke<Track[]>("get_playlist_tracks", { id });
}

export function addToPlaylist(
  playlistId: number,
  trackIds: number[],
): Promise<void> {
  return invoke("add_to_playlist", { playlistId, trackIds });
}

export function removeFromPlaylist(
  playlistId: number,
  position: number,
): Promise<void> {
  return invoke("remove_from_playlist", { playlistId, position });
}

export function setPlaylistOrder(
  playlistId: number,
  trackIds: number[],
): Promise<void> {
  return invoke("set_playlist_order", { playlistId, trackIds });
}
