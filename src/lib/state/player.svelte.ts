// Global player state as a rune-based module — no store library needed.
// One Audio element lives here for the whole app; it never mounts into the
// DOM (audio doesn't need to be in the document to play).

import { convertFileSrc } from "@tauri-apps/api/core";
import { SvelteSet } from "svelte/reactivity";
import type { RepeatMode, Track } from "../types";

const audio = new Audio();

export const player = $state({
  queue: [] as Track[],
  /** Index into `queue` of the current track, or -1 when nothing is loaded. */
  index: -1,
  isPlaying: false,
  /** Playback position in seconds (driven by `timeupdate`). */
  position: 0,
  duration: 0,
  volume: 1,
  shuffle: false,
  repeat: "off" as RepeatMode,
  /** Tracks whose files failed to load/decode (missing file or unsupported codec). */
  errorTrackIds: new SvelteSet<number>(),
});

/** The queue order before shuffle was switched on, so we can restore it. */
let unshuffled: Track[] | null = null;

/** Stop skipping forward if several tracks in a row fail to load. */
let consecutiveErrors = 0;

export function currentTrack(): Track | null {
  return player.index >= 0 ? (player.queue[player.index] ?? null) : null;
}

// ---- Audio element wiring ----

audio.addEventListener("timeupdate", () => {
  player.position = audio.currentTime;
});

audio.addEventListener("loadedmetadata", () => {
  player.duration = audio.duration;
  consecutiveErrors = 0;
  // A previously failing track that loads now (e.g. the file came back)
  // sheds its "unplayable" badge.
  const track = currentTrack();
  if (track) player.errorTrackIds.delete(track.id);
});

audio.addEventListener("play", () => (player.isPlaying = true));
audio.addEventListener("pause", () => (player.isPlaying = false));

audio.addEventListener("ended", () => {
  if (player.repeat === "one") {
    audio.currentTime = 0;
    void audio.play();
  } else {
    next(true);
  }
});

// The codec gap (PLAN 3.4): FLAC/Ogg support depends on the OS webview, and
// files can simply go missing. Mark the track and move on instead of dying
// silently.
audio.addEventListener("error", () => {
  const track = currentTrack();
  if (track) player.errorTrackIds.add(track.id);
  player.isPlaying = false;
  consecutiveErrors += 1;
  if (consecutiveErrors < 5) next(true);
});

// ---- Internal helpers ----

function load(track: Track, autoplay: boolean) {
  player.position = 0;
  player.duration = track.durationSecs ?? 0;
  audio.src = convertFileSrc(track.path);
  if (autoplay) {
    audio.play().catch(() => {
      // `error` listener above handles decode failures; this catches
      // play() interruptions, which we can safely ignore.
    });
  }
  updateMediaSession(track);
}

function shuffled<T>(items: T[]): T[] {
  // Fisher–Yates.
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

// ---- Public API (UI buttons, tray, media keys all converge here) ----

/** Play `tracks[startIndex]`, making `tracks` the new queue. */
export function playContext(tracks: Track[], startIndex: number) {
  if (tracks.length === 0) return;
  consecutiveErrors = 0;
  if (player.shuffle) {
    const start = tracks[startIndex];
    const rest = tracks.filter((_, i) => i !== startIndex);
    unshuffled = [...tracks];
    player.queue = [start, ...shuffled(rest)];
    player.index = 0;
  } else {
    unshuffled = null;
    player.queue = [...tracks];
    player.index = startIndex;
  }
  load(player.queue[player.index], true);
}

/** Jump to a specific queue position. */
export function playQueueAt(index: number) {
  if (index < 0 || index >= player.queue.length) return;
  consecutiveErrors = 0;
  player.index = index;
  load(player.queue[index], true);
}

/** Append tracks to the queue; starts playback if nothing is queued. */
export function enqueue(tracks: Track[]) {
  if (tracks.length === 0) return;
  const wasEmpty = player.queue.length === 0;
  player.queue = [...player.queue, ...tracks];
  if (wasEmpty) playQueueAt(0);
}

export function togglePlay() {
  if (!currentTrack()) {
    if (player.queue.length > 0) playQueueAt(0);
    return;
  }
  if (audio.paused) void audio.play();
  else audio.pause();
}

export function next(autoplay = true) {
  if (player.queue.length === 0) return;
  if (player.index + 1 < player.queue.length) {
    player.index += 1;
  } else if (player.repeat === "all") {
    player.index = 0;
  } else {
    // End of queue: stop.
    audio.pause();
    audio.currentTime = 0;
    player.position = 0;
    return;
  }
  load(player.queue[player.index], autoplay || player.isPlaying);
}

export function prev() {
  if (player.queue.length === 0) return;
  // Convention: early in a track, go to the previous one; later, restart it.
  if (audio.currentTime > 3 || player.index === 0) {
    audio.currentTime = 0;
    return;
  }
  player.index -= 1;
  load(player.queue[player.index], true);
}

export function seek(seconds: number) {
  audio.currentTime = seconds;
  player.position = seconds;
}

export function setVolume(volume: number) {
  const v = Math.min(1, Math.max(0, volume));
  player.volume = v;
  audio.volume = v;
}

export function toggleShuffle() {
  player.shuffle = !player.shuffle;
  const current = currentTrack();
  if (player.shuffle) {
    // Shuffle everything after the current track; current stays put.
    unshuffled = [...player.queue];
    if (current) {
      const rest = player.queue.filter((_, i) => i !== player.index);
      player.queue = [current, ...shuffled(rest)];
      player.index = 0;
    } else {
      player.queue = shuffled(player.queue);
    }
  } else if (unshuffled) {
    player.queue = unshuffled;
    player.index = current
      ? Math.max(0, player.queue.findIndex((t) => t.id === current.id))
      : -1;
    unshuffled = null;
  }
}

export function cycleRepeat() {
  const order: RepeatMode[] = ["off", "all", "one"];
  player.repeat = order[(order.indexOf(player.repeat) + 1) % order.length];
}

// ---- Media Session (lock screen / OS media controls, where supported) ----

function updateMediaSession(track: Track) {
  if (!("mediaSession" in navigator)) return;
  navigator.mediaSession.metadata = new MediaMetadata({
    title: track.title,
    artist: track.artist ?? "",
    album: track.album ?? "",
    artwork: track.coverPath
      ? [{ src: convertFileSrc(track.coverPath) }]
      : [],
  });
}

if ("mediaSession" in navigator) {
  navigator.mediaSession.setActionHandler("play", () => void audio.play());
  navigator.mediaSession.setActionHandler("pause", () => audio.pause());
  navigator.mediaSession.setActionHandler("previoustrack", () => prev());
  navigator.mediaSession.setActionHandler("nexttrack", () => next());
  navigator.mediaSession.setActionHandler("seekto", (e) => {
    if (e.seekTime != null) seek(e.seekTime);
  });
}
