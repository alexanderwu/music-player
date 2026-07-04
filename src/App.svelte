<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import LibraryView from "./lib/components/LibraryView.svelte";
  import PlaylistView from "./lib/components/PlaylistView.svelte";
  import SettingsView from "./lib/components/SettingsView.svelte";
  import PlayerBar from "./lib/components/PlayerBar.svelte";
  import { handleDroppedPaths } from "./lib/ipc";
  import { initLibraryListeners, loadLibrary } from "./lib/state/library.svelte";
  import { enqueue, next, prev, togglePlay } from "./lib/state/player.svelte";
  import { refreshPlaylists } from "./lib/state/playlists.svelte";
  import { initSettings } from "./lib/state/settings.svelte";
  import { focusSearch, ui } from "./lib/state/ui.svelte";

  onMount(() => {
    const cleanups: (() => void)[] = [];

    void (async () => {
      await initSettings();
      await initLibraryListeners();
      await loadLibrary();
      await refreshPlaylists();

      // Tray menu clicks and media keys both funnel into the same player
      // functions the on-screen buttons use — one state module, many inputs.
      cleanups.push(await listen("media:play-pause", () => togglePlay()));
      cleanups.push(await listen("media:next", () => next()));
      cleanups.push(await listen("media:prev", () => prev()));

      // Native drag & drop: Tauri delivers real file paths (web drop
      // events never expose paths). Files → queue; folders → scan.
      cleanups.push(
        await getCurrentWebview().onDragDropEvent(async (event) => {
          const type = event.payload.type;
          if (type === "enter" || type === "over") {
            ui.dropActive = true;
          } else if (type === "leave") {
            ui.dropActive = false;
          } else if (type === "drop") {
            ui.dropActive = false;
            const result = await handleDroppedPaths(event.payload.paths);
            enqueue(result.queued);
          }
        }),
      );
    })();

    return () => cleanups.forEach((fn) => fn());
  });

  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const typing =
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT" ||
      target.isContentEditable;

    if (e.code === "Space" && !typing) {
      e.preventDefault();
      togglePlay();
    } else if (e.key === "/" && !typing) {
      e.preventDefault();
      focusSearch();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  <Sidebar />
  <main>
    {#if ui.view === "library"}
      <LibraryView />
    {:else if ui.view === "playlist"}
      <PlaylistView />
    {:else}
      <SettingsView />
    {/if}
  </main>
  <PlayerBar />

  {#if ui.dropActive}
    <div class="drop-overlay">
      <div class="drop-card">Drop music files or folders to add them</div>
    </div>
  {/if}
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: 220px 1fr;
    grid-template-rows: 1fr auto;
    grid-template-areas:
      "sidebar main"
      "player player";
    height: 100vh;
    overflow: hidden;
  }
  .app > :global(nav) {
    grid-area: sidebar;
  }
  main {
    grid-area: main;
    min-height: 0;
    overflow: hidden;
  }
  .app > :global(footer) {
    grid-area: player;
  }
  .drop-overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgb(0 0 0 / 0.4);
    pointer-events: none;
  }
  .drop-card {
    padding: 1.5rem 2.5rem;
    border: 2px dashed var(--accent);
    border-radius: 12px;
    background: var(--surface);
    font-size: 1.1rem;
  }
</style>
