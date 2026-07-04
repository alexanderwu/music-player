// Playlist state. The frontend never sees SQL — it talks to the typed
// command API in ipc.ts, the same layering as a web app's API server.

import * as ipc from "../ipc";
import type { Playlist, Track } from "../types";

export const playlists = $state({
  all: [] as Playlist[],
  /** Playlist currently open in the main view, or null for the library. */
  activeId: null as number | null,
  activeTracks: [] as Track[],
});

export function activePlaylist(): Playlist | null {
  return playlists.all.find((p) => p.id === playlists.activeId) ?? null;
}

export async function refreshPlaylists() {
  playlists.all = await ipc.getPlaylists();
}

export async function openPlaylist(id: number) {
  playlists.activeId = id;
  playlists.activeTracks = await ipc.getPlaylistTracks(id);
}

export async function createPlaylist(name: string): Promise<Playlist> {
  const playlist = await ipc.createPlaylist(name);
  await refreshPlaylists();
  return playlist;
}

export async function renamePlaylist(id: number, name: string) {
  await ipc.renamePlaylist(id, name);
  await refreshPlaylists();
}

export async function deletePlaylist(id: number) {
  await ipc.deletePlaylist(id);
  if (playlists.activeId === id) {
    playlists.activeId = null;
    playlists.activeTracks = [];
  }
  await refreshPlaylists();
}

export async function addToPlaylist(playlistId: number, trackIds: number[]) {
  await ipc.addToPlaylist(playlistId, trackIds);
  await refreshPlaylists();
  if (playlists.activeId === playlistId) await openPlaylist(playlistId);
}

export async function removeAt(position: number) {
  if (playlists.activeId == null) return;
  await ipc.removeFromPlaylist(playlists.activeId, position);
  await refreshPlaylists();
  await openPlaylist(playlists.activeId);
}

/** Drag-to-reorder: move the row at `from` to `to`, then persist the order. */
export async function reorderActive(from: number, to: number) {
  if (playlists.activeId == null || from === to) return;
  const tracks = [...playlists.activeTracks];
  const [moved] = tracks.splice(from, 1);
  tracks.splice(to, 0, moved);
  playlists.activeTracks = tracks; // optimistic
  await ipc.setPlaylistOrder(
    playlists.activeId,
    tracks.map((t) => t.id),
  );
}
