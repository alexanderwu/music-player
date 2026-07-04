# PLAN.md — Building a Desktop Music Player with Tauri v2 + Svelte 5

This is a learning-first plan. Every phase explains **what** to build, **why** it's built that way, and **what the alternatives were**, so by the end you understand not just this app but the general shape of desktop app development.

**Profile this plan assumes:** you're comfortable with web development (HTML/CSS/JS), but new to Rust, Tauri, and possibly Svelte. Rust code will be introduced gradually and explained as it appears.

---

## 1. What we're building

A full-featured desktop music player:

- **Library**: point the app at a music folder; it scans it, reads metadata tags (title, artist, album, artwork), and shows a searchable/sortable library.
- **Playback**: play/pause/seek, next/previous, volume, a play queue, shuffle/repeat.
- **Playlists**: create, edit, persist.
- **Persistence**: the library and playlists survive restarts (SQLite).
- **Desktop integration**: system tray with playback controls, media-key support, drag-and-drop files onto the window, remembering window size/position.
- **Distribution**: a real installable app (`.dmg` / `.msi` / `.deb`/`.AppImage`).

This feature set is deliberately chosen because it forces you through every core desktop-app concept at least once: native file access, background work, inter-process communication, local databases, OS integration, and packaging.

---

## 2. The big picture: how a Tauri app is shaped

Before any decisions, you need the mental model.

A Tauri app is **two programs in one process**:

```
┌──────────────────────────────────────────────────────┐
│  Your app binary (Rust)                              │
│                                                      │
│  ┌────────────────┐         ┌──────────────────────┐ │
│  │  Rust "core"   │  IPC    │  OS WebView           │ │
│  │                │◄───────►│                       │ │
│  │  - file system │ invoke/ │  Your Svelte app      │ │
│  │  - SQLite      │ events  │  - UI                 │ │
│  │  - scanning    │         │  - <audio> playback   │ │
│  │  - tray, keys  │         │  - state (runes)      │ │
│  └────────────────┘         └──────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

- The **Rust side** (called the "core" or "backend", lives in `src-tauri/`) is a normal native program. It can do anything a desktop app can: read any file, spawn threads, talk to the OS.
- The **frontend** (lives in `src/`) is a web app rendered in the **operating system's built-in webview** (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux). It is sandboxed like a web page — it *cannot* touch the filesystem directly.
- They talk over **IPC** (inter-process communication):
  - Frontend → Rust: `invoke("command_name", args)` calls a Rust function and awaits its result. Think of it as `fetch()` to a local API.
  - Rust → Frontend: **events** (`app.emit(...)` / `listen(...)`). Think server-sent events / websockets.

**The core design skill in Tauri is deciding which side each piece of work belongs on.** Rule of thumb: UI and anything the web platform already does well → frontend. Filesystem, heavy computation, OS integration → Rust. We'll apply this rule repeatedly and call it out each time.

---

## 3. Stack decisions and their trade-offs

### 3.1 Why Tauri (vs Electron, and others)

| | Tauri v2 | Electron | Flutter/others |
|---|---|---|---|
| Renderer | OS webview | Bundled Chromium | Custom (Skia) |
| Backend language | Rust | Node.js | Dart |
| Typical bundle | ~5–15 MB | ~100–200 MB | ~30–50 MB |
| RAM usage | Low | High (full Chromium per app) | Medium |
| Rendering consistency | Varies per OS | Identical everywhere | Identical everywhere |

- **Electron** bundles an entire Chromium browser with every app. That's why Slack and VS Code are hundreds of MB. The upside: your app renders identically on every OS and you write backend code in JavaScript. The downside: size, memory, and a large attack surface.
- **Tauri** reuses the webview the OS already ships. Result: tiny binaries and low memory. The costs: (1) rendering/codec differences between platforms (Safari-based WKWebView on macOS behaves differently than Chromium-based WebView2 on Windows — we'll hit this with audio codecs, see 3.4), and (2) the backend is Rust, which has a real learning curve.
- **Flutter et al.** skip HTML/CSS entirely — not useful for us since the goal includes leveraging your web skills.

**Why Tauri v2 specifically:** v2 (stable since late 2024) rewrote the permission system into *capabilities* (explicit, auditable grants of what the frontend may ask the core to do — you'll edit these files yourself in Phase 2), moved most built-in functionality into official *plugins*, and added mobile support. All current docs and plugins target v2; v1 is legacy.

**The security model is worth understanding early:** because the frontend is "just a web page", Tauri treats it as semi-trusted. Every native ability the frontend uses (open a dialog, read a file, receive events) must be declared in `src-tauri/capabilities/*.json`. This is annoying at first and then you realize it's the whole point — a compromised frontend (e.g. via a malicious mp3 tag rendered as HTML) can only do what you allowed.

### 3.2 Why Svelte 5 (vs React, Vue)

- **React** re-renders components and diffs a virtual DOM. **Svelte is a compiler**: your components compile to imperative DOM updates, so there's no framework runtime doing diffing at runtime. For a desktop app this means snappier UI and a smaller bundle — and pedagogically, Svelte's output is closer to "what the browser actually does."
- **Svelte 5's runes** (`$state`, `$derived`, `$effect`) are explicit reactivity primitives. Compared to React: `$state` ≈ `useState` but without re-running the whole component; `$derived` ≈ `useMemo` without dependency arrays; `$effect` ≈ `useEffect` without dependency arrays. Crucially, runes work in plain `.svelte.ts` files too — which is how we'll do global state (player state, library) *without* a state-management library like Redux/Zustand. That's a real architectural lesson: fine-grained reactivity makes "global store" trivial.
- **TypeScript** matters most at the **IPC boundary**. `invoke()` returns `Promise<unknown>` — the compiler can't see Rust. We'll define shared TS types mirroring the Rust structs, so the boundary is at least *documented* and checked on the TS side. (Trade-off note: tools like `specta`/`tauri-specta` can auto-generate TS types from Rust; we'll do it manually first because doing it by hand teaches you what the boundary actually is, then mention automation as an upgrade.)

- **Plain Vite + Svelte, not SvelteKit.** SvelteKit is Svelte's app framework with routing, server-side rendering, and API endpoints — all built around a *server*. A Tauri app has no server; it's static files loaded into a webview. SvelteKit can run in static mode, but you'd spend effort disabling its server features. A single-window music player doesn't even need a URL router — "which view is showing" is just a piece of state. Choosing plain Vite keeps the mental model honest: *this is not a website*.

### 3.3 Why Vite

Non-decision, really: it's the default toolchain for Svelte and what `create-tauri-app` scaffolds. Relevant detail: in development, Tauri points the webview at Vite's dev server (`http://localhost:1420`) so you get hot-module reload; in production the built static files are embedded *inside the Rust binary*. Understanding that dev/prod difference explains several things later (like why file paths behave differently, and why we need `beforeDevCommand`/`beforeBuildCommand` in `tauri.conf.json`).

### 3.4 The audio engine: webview `<audio>` (the decision you made, and its consequences)

Two credible architectures:

1. **Play audio in the frontend** with the HTML5 `<audio>` element. The webview needs to read audio files from disk, which sandboxed web pages can't do — Tauri's **asset protocol** solves this: `convertFileSrc("/path/to/song.mp3")` returns an `asset://` URL the webview may load, *if* that path is allowed by the capability config (`assetProtocol` scope).
2. **Play audio in Rust** with the `rodio` crate. The frontend becomes a remote control sending play/pause/seek commands.

We chose **(1)**, and here's the full trade-off so you know what you bought:

- ✅ You get seek, buffering, progress (`timeupdate`), volume, and playback-rate handling for free — the browser has 20 years of polish here. With rodio you'd hand-implement seek and progress reporting over IPC, which is genuinely fiddly.
- ✅ It teaches *the* canonical Tauri pattern: lean on the web platform, keep Rust for what the web can't do.
- ✅ Media Session API (`navigator.mediaSession`) may give OS "now playing" integration for free on some platforms.
- ⚠️ **Codec support depends on the OS webview.** MP3, AAC/M4A, and WAV work everywhere. FLAC works on WebView2 (Windows) and modern WKWebView (macOS 14+), Ogg Vorbis works on WebView2/WebKitGTK but historically *not* WKWebView. Rodio would decode uniformly on all platforms. We'll accept this and note the per-format reality in the UI (Phase 4 includes a graceful "can't play this format" path).
- ⚠️ Audio stops if the webview process dies — irrelevant for us, but it's why "background audio" apps sometimes choose the Rust route.

### 3.5 Where the data work happens: Rust

- **Scanning + tag reading in Rust** with `walkdir` (recursive directory traversal) + `lofty` (reads ID3v2/Vorbis/MP4 tags uniformly across formats). Could we read tags in JS with `music-metadata`? Yes — but the frontend would need broad filesystem read permissions (bad for the security model), and scanning 10,000 files means 10,000 IPC round-trips or granting the webview raw file access. In Rust it's a tight native loop on a background thread that streams progress events to the UI. This is the clearest "right side of the boundary" call in the app.
- **SQLite via `rusqlite` in Rust** (not `tauri-plugin-sql`). The plugin lets the *frontend* run SQL strings directly, which is quick to start with but has two smells: your schema/queries live in UI code, and the DB becomes writable by the semi-trusted frontend. With `rusqlite`, the database is owned by the core, and the frontend gets a small, typed command API (`get_tracks`, `search_library`, `create_playlist`, ...). That's the same layering as a web app's API server, applied to the desktop — the pattern is the lesson. Trade-off: more Rust to write; we keep it manageable.
  - Why SQLite at all (vs a JSON file)? A JSON library file is fine at 100 tracks and painful at 20,000 (load-all-into-memory, no indexed search, corruption on partial writes). SQLite gives indexed queries, atomic transactions, and it's *the* embedded database — knowing it transfers everywhere. We'll note the JSON alternative in Phase 5 so you see where the crossover point is.
- **Settings via `tauri-plugin-store`** (a small JSON key-value store) — because settings *are* the 100-track case: tiny, no queries. Using both teaches you to match storage to data shape.

### 3.6 Desktop integration pieces

- **Folder picker**: `tauri-plugin-dialog`. Never build file pickers in HTML — users expect the native one, and the native one is how the OS grants your app access to the chosen folder.
- **System tray**: Tauri's built-in `TrayIcon` API (Rust side) with a menu (Play/Pause, Next, Quit). Teaches Rust→frontend events in the opposite direction from usual.
- **Media keys**: `tauri-plugin-global-shortcut` binding `MediaPlayPause`, `MediaTrackNext`, `MediaTrackPrevious`. Trade-off note: the *deluxe* approach is the `souvlaki` crate (real OS media sessions: MPRIS on Linux, SMTC on Windows, Now Playing on macOS with artwork on the lock screen) — listed as a stretch goal because it's more Rust than it's worth on the first pass, and `navigator.mediaSession` from 3.4 may cover parts of it.
- **Drag & drop**: Tauri intercepts native file drops and delivers them as events with real file paths (web-standard drop events don't expose paths, for security). Config flag + one listener.
- **Window state (size/position memory)**: `tauri-plugin-window-state`. One line; teaches you the plugin ecosystem exists so you don't hand-roll solved problems.

---

## 4. Prerequisites & setup (Phase 0)

**Install:**
1. **Rust** via `rustup` (installs `cargo`, Rust's npm-equivalent). You will not need to *know* much Rust to start — you'll learn by editing working code.
2. **Node.js** ≥ 20 (you have this covered).
3. **OS build deps** — Linux needs `libwebkit2gtk-4.1-dev` and friends; macOS needs Xcode Command Line Tools; Windows needs the Build Tools for Visual Studio + WebView2 (preinstalled on Win 11). Follow https://tauri.app/start/prerequisites/ exactly; this is the step where 90% of "Tauri doesn't work" issues live.

**Scaffold:**
```bash
pnpm create tauri-app@latest music-player --template svelte-ts
cd music-player && pnpm install && pnpm tauri dev
```

**Checkpoint:** a native window opens showing the starter page, and editing `src/App.svelte` hot-reloads it. 

**Then take 30 minutes to read the scaffold** — this is the actual lesson of Phase 0:
- `src/` — the Svelte app (plain web project).
- `src-tauri/tauri.conf.json` — app identity, window config, dev/build commands, bundling.
- `src-tauri/src/lib.rs` — the Rust entry: notice the `greet` command and the `invoke_handler` registering it. Delete-and-rebuild the greet demo once; watching it break and fixing it teaches the command wiring faster than any doc.
- `src-tauri/capabilities/default.json` — the permission grants discussed in 3.1.
- `src-tauri/Cargo.toml` — Rust's `package.json`.

---

## 5. Build phases

Each phase ends with a **checkpoint** (something observable working) and a **commit**. Keep commits at phase-or-smaller granularity — when something breaks in Phase 6, `git bisect`-style thinking will save you.

### Phase 1 — Skeleton UI and app-wide state (frontend only)

**Goal:** static-data version of the whole UI. No Tauri APIs yet — prove out Svelte 5 first.

1. Sketch the layout: sidebar (Library / Playlists), main content area (track table), player bar pinned to the bottom (artwork, title/artist, transport buttons, seek slider, volume).
2. Create the shared-state modules as `.svelte.ts` files — this is the app's spine:
   - `src/lib/state/player.svelte.ts` — current track, queue, position, `isPlaying`, volume, shuffle/repeat mode.
   - `src/lib/state/library.svelte.ts` — track list, scan status, search text, `$derived` filtered/sorted view.
3. Define the core TS types (`Track`, `Playlist`) in `src/lib/types.ts`. These will later mirror Rust structs exactly — write them thoughtfully now.
4. Hardcode ~20 fake tracks; wire the table, search box, and (non-functional) player bar to the state modules.

**Rationale:** building UI against fake data first separates "is my UI wrong?" from "is my IPC wrong?" forever after. It's also where you internalize runes: e.g. the filtered track list is a `$derived` of `tracks` + `searchText`, and the table updates with zero glue code.

**Decision to make here, consciously:** no component library, plain CSS (or a sprinkle of CSS variables for theming). Trade-off: component libraries (skeleton, shadcn-svelte) speed you up but hide the CSS you're here to stay fluent in. A music player's UI is small enough to hand-roll.

**Checkpoint:** the app looks like a music player and search filters the fake list.

### Phase 2 — First real IPC: pick a folder, scan it in Rust

**Goal:** the golden thread — frontend button → Rust work → results in UI.

1. Add `tauri-plugin-dialog`; note the *three* registration points every plugin has (Cargo.toml dependency, `.plugin(...)` in `lib.rs`, permission in `capabilities/`). Add an npm `@tauri-apps/plugin-dialog` package for the JS bindings. This dance is confusing exactly once.
2. Write your first real Rust command in `lib.rs`:
   ```rust
   #[tauri::command]
   fn scan_folder(path: String) -> Result<Vec<Track>, String> { ... }
   ```
   Concepts you'll meet, each worth a moment: `#[tauri::command]` (a macro generating the IPC glue), `Result<T, E>` (Rust's errors-as-values — the `Err` becomes a rejected Promise in JS!), `#[derive(Serialize)]` on the `Track` struct (how Rust structs become JSON), and registering the command in `invoke_handler`.
3. Use `walkdir` to collect files with audio extensions; return paths + filenames only (no tags yet — one new thing at a time).
4. Frontend: "Add folder" button → `open()` dialog → `invoke<Track[]>("scan_folder", { path })` → replace the fake tracks.

**Gotcha worth learning deliberately:** Rust uses `snake_case` fields; JS expects `camelCase`. Fix with `#[serde(rename_all = "camelCase")]` on the struct. Congratulations, you've now debugged your first IPC-boundary mismatch on purpose instead of by surprise.

**Checkpoint:** your real music folder's filenames appear in the table.

### Phase 3 — Metadata, artwork, and a scanner that doesn't freeze

**Goal:** real tags, and the difference between blocking and background work.

1. Add `lofty`; in the scan loop, read title/artist/album/duration/track number per file, falling back to the filename when tags are missing (real libraries are dirty — handle it now, not later).
2. **Feel the problem first:** scan a big folder with the naive synchronous command and watch the UI sit frozen on the spinner with no feedback. *Then* fix it:
   - Make the command `async` and spawn the scan on a background thread (`tauri::async_runtime::spawn_blocking` — CPU/IO-heavy work doesn't belong on the async runtime's core threads).
   - Emit progress events from the loop: `app.emit("scan:progress", ScanProgress { done, total, current_file })`.
   - Frontend: `listen("scan:progress", ...)` updates a progress bar in the library state module.
   
   This before/after is the single most transferable desktop-app lesson in the plan: **UI thread sacred, work elsewhere, progress via events.** Every serious desktop app is shaped like this.
3. Artwork: extract embedded cover art with lofty, write to a cache dir (`app_data_dir/covers/{hash}.jpg`), return the path; frontend displays it via `convertFileSrc`. Rationale for caching on disk vs returning base64 blobs: album art repeats per album, is big, and disk-cached files let the webview cache them too.

**Checkpoint:** scan of a large folder shows live progress, UI stays responsive, and the table shows real titles/artists/artwork.

### Phase 4 — Playback

**Goal:** it plays music.

1. Enable the asset protocol and scope it (capabilities) so the webview may load your music folders. Note what you're doing security-wise: granting read access to *those trees only*, not the disk.
2. In `player.svelte.ts`, manage one `new Audio()` instance (an element that never mounts — audio doesn't need to be in the DOM). Set `audio.src = convertFileSrc(track.path)` on track change.
3. Wire the transport: play/pause, `timeupdate` → seek slider position, slider drag → `audio.currentTime`, `ended` → advance queue, volume. Shuffle (Fisher–Yates over the queue) and repeat modes live purely in the state module.
4. Handle the codec gap from 3.4: on `audio.error`, show a per-track "format unsupported" state instead of dying silently.
5. Wire `navigator.mediaSession` metadata + handlers — a few lines, possibly free lock-screen integration.

**Checkpoint:** double-click a track, hear music, seek around, queue advances. This is the dopamine phase.

### Phase 5 — Persistence (SQLite)

**Goal:** the library survives restarts; frontend talks to a typed API, not a database.

1. Add `rusqlite` (bundled feature, so SQLite compiles in — no system dependency). Create the DB in `app_data_dir()` (ask Tauri for the correct per-OS location — never hardcode; this is `~/Library/Application Support/...` on macOS, `%APPDATA%` on Windows, `~/.local/share` on Linux).
2. Schema: `tracks`, `folders`, `playlists`, `playlist_tracks` (ordered via a position column). Write it as a `schema.sql` you execute on startup; add a trivial `user_version`-based migration check so you learn the migration concept while it's cheap.
3. Introduce **managed state** — Tauri's DI mechanism: wrap the connection in `Mutex<Connection>`, register with `.manage(...)`, receive it in commands as `State<...>`. (Why the Mutex: commands can run concurrently; SQLite connections aren't thread-safe to share. One serialized connection is plenty for this app — note the alternative, a pool, for write-heavy apps.)
4. Rework the scanner to upsert into the DB; add commands `get_tracks`, `search_tracks`, playlist CRUD. On startup the frontend calls `get_tracks` — the library is just *there*.
5. Move search into SQL (`WHERE ... LIKE`) — and discuss the trade-off you just navigated: in-memory filtering (Phase 1) is simpler and instant at small scale; DB queries win at large scale. You've now implemented both and can feel where the line is.
6. Playlists UI: create/rename/delete, add-to-playlist context menu, drag-to-reorder (position column).

**Checkpoint:** restart the app — library, playlists, and last-used folders are all still there.

### Phase 6 — Desktop integration

**Goal:** the things that make it feel like an app, not a page.

1. **Tray** (Rust): icon + menu (Play/Pause, Next, Previous, Show, Quit). Menu clicks emit events to the frontend, which calls the same player-state functions the buttons use. Also: close button hides to tray instead of quitting (intercept `CloseRequested`) — with a real Quit in the tray. Discuss: users have opinions about close-to-tray; make it a setting later.
2. **Media keys**: `tauri-plugin-global-shortcut` for `MediaPlayPause` / `MediaTrackNext` / `MediaTrackPrevious` → same events. Note the layering payoff: tray, media keys, and UI buttons all converge on one state module — because Phase 1 put state in one place.
3. **Drag & drop**: enable file drop, listen for dropped paths, route files → add to queue, folders → offer to scan.
4. **Window state**: add `tauri-plugin-window-state`. Done. (The lesson is that it exists.)
5. **Settings**: `tauri-plugin-store` for volume, close-to-tray preference, theme.

**Checkpoint:** control playback from the tray and keyboard with the window hidden; drop an mp3 onto the window and it queues.

### Phase 7 — Polish pass

- Keyboard shortcuts in-app (space = play/pause, `/` = focus search).
- Loading/empty/error states everywhere (empty library, missing files — files move! handle a track whose path no longer exists).
- Virtualized track list if your library is big (10k DOM rows will hurt; `svelte-virtual-list` or hand-roll a simple windower — hand-rolling one is a great optional exercise).
- Light/dark theme via CSS variables + `prefers-color-scheme`.

### Phase 8 — Ship it

**Goal:** an artifact you can install and give to a friend.

1. `pnpm tauri build` — produces platform bundles (`.dmg`, `.msi`/NSIS, `.deb`/`.AppImage`). Cross-compilation is *not* really a thing here (each OS builds its own) — that's what CI matrices are for; add a GitHub Actions workflow using `tauri-apps/tauri-action` as the canonical solution.
2. App icon: one 1024×1024 PNG → `pnpm tauri icon` generates every platform format.
3. Read the output sizes and smile (single-digit MB).
4. **Learn what you're skipping and why it exists:** code signing & notarization (macOS Gatekeeper / Windows SmartScreen will warn on unsigned apps — signing costs money/certificates; fine to skip for personal use, mandatory for real distribution) and auto-updates (`tauri-plugin-updater`, needs signing) — both stretch goals.

**Checkpoint:** installed app launches from the OS launcher, plays music, sits in the tray.

---

## 6. Stretch goals (each one a self-contained lesson)

- **`souvlaki` OS media sessions** — proper now-playing with artwork on lock screens; deeper Rust + platform APIs.
- **Audio visualizer** — Web Audio API `AnalyserNode` + canvas; pure frontend, very satisfying.
- **Gapless playback** — preload the next track in a second `Audio` element; teaches you why "simple" player features are hard.
- **Watch folders for changes** — the `notify` crate; long-running Rust background tasks.
- **`tauri-specta`** — auto-generate the TS types for commands/events from Rust; removes the manual type-mirroring from Phase 2.
- **Auto-updates + signing** — the full grown-up distribution story.

---

## 7. Reference map

| Topic | Where |
|---|---|
| Tauri v2 docs | https://tauri.app/ (concepts → "Inter-Process Communication" is the key page) |
| Capabilities/permissions | https://tauri.app/security/capabilities/ |
| Official plugins | https://tauri.app/plugin/ |
| Svelte 5 runes | https://svelte.dev/docs/svelte/what-are-runes |
| lofty (tags) | https://docs.rs/lofty |
| rusqlite | https://docs.rs/rusqlite |
| Rust, when you want depth | https://doc.rust-lang.org/book/ (ch. 1–10 covers everything this app needs) |

## 8. Suggested rhythm

Phases 0–2 in your first sitting or two (setup pain + first IPC is one continuous learning arc — don't break in the middle of Phase 2). Phases 3–5 are the meat; take them one per session. 6–8 are independent of each other and can be done in any order or mood.
