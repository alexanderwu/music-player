# Music Player

A desktop music player built with **Tauri v2** and **Svelte 5**, following the
learning-first plan in [PLAN.md](PLAN.md).

## Features

- **Library** — point the app at a music folder; a background Rust scanner
  reads tags (title, artist, album, artwork) with live progress, and the
  library is searchable and sortable.
- **Playback** — play/pause/seek, next/previous, volume, play queue,
  shuffle and repeat, via the webview `<audio>` element and the scoped
  asset protocol.
- **Playlists** — create, rename, delete, add via right-click,
  drag-to-reorder; persisted in SQLite.
- **Persistence** — SQLite (`rusqlite`, owned by the Rust core behind a
  typed command API) for the library and playlists; `tauri-plugin-store`
  for settings.
- **Desktop integration** — system tray with playback controls,
  close-to-tray (configurable), media keys, drag & drop of files/folders
  onto the window, window size/position memory.
- **Polish** — virtualized track list, keyboard shortcuts
  (<kbd>Space</kbd> play/pause, <kbd>/</kbd> search), light/dark theme,
  graceful handling of missing files and unsupported codecs.

## Development

Prerequisites: [Rust](https://rustup.rs), Node.js ≥ 20, and the Tauri
[platform dependencies](https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev
```

Useful scripts:

| Command             | What it does                          |
| ------------------- | ------------------------------------- |
| `npm run tauri dev` | Run the app with hot reload           |
| `npm run check`     | Type-check the frontend (svelte-check)|
| `npm run build`     | Build the frontend bundle             |
| `npm run tauri build` | Build installable bundles (`.deb`/`.AppImage`/`.dmg`/`.msi`) |

## Architecture

Two programs in one process (see PLAN.md §2):

- `src/` — the Svelte 5 frontend. Global state lives in rune-based
  `.svelte.ts` modules (`src/lib/state/`); the IPC boundary is typed in
  `src/lib/ipc.ts`, mirroring the Rust structs.
- `src-tauri/` — the Rust core: folder scanning (`walkdir` + `lofty`),
  SQLite (`rusqlite`), tray, media keys, and the capability/asset-protocol
  security config.

Releases are built by the CI matrix in
[.github/workflows/release.yml](.github/workflows/release.yml) — push a
`v*` tag to produce draft-release installers for macOS, Windows, and Linux.
