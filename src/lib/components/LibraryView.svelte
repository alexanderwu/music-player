<script lang="ts">
  import TrackTable from "./TrackTable.svelte";
  import {
    addFolder,
    library,
    setSearch,
    setSort,
    sortedTracks,
  } from "../state/library.svelte";
  import { playContext } from "../state/player.svelte";
  import { ui } from "../state/ui.svelte";

  let searchInput: HTMLInputElement | undefined = $state();
  let tracks = $derived(sortedTracks());

  // "/" anywhere focuses search (see App.svelte's key handler).
  $effect(() => {
    if (ui.searchFocusTick > 0) searchInput?.focus();
  });
</script>

<div class="library">
  <header>
    <input
      bind:this={searchInput}
      class="search"
      type="search"
      placeholder="Search title, artist, album…  ( / )"
      value={library.searchText}
      oninput={(e) => setSearch(e.currentTarget.value)}
    />
    <button class="primary" onclick={() => void addFolder()} disabled={library.scanning}>
      Add folder
    </button>
  </header>

  {#if library.scanning}
    <div class="scan-status">
      {#if library.progress}
        <progress value={library.progress.done} max={library.progress.total}></progress>
        <span class="scan-text">
          Scanning {library.progress.done}/{library.progress.total} —
          {library.progress.currentFile}
        </span>
      {:else}
        <progress></progress>
        <span class="scan-text">Scanning…</span>
      {/if}
    </div>
  {/if}

  {#if tracks.length === 0}
    <div class="empty">
      {#if !library.loaded}
        <p>Loading library…</p>
      {:else if library.searchText}
        <p>No tracks match “{library.searchText}”.</p>
      {:else if !library.scanning}
        <p class="empty-icon">♫</p>
        <p>Your library is empty.</p>
        <button class="primary" onclick={() => void addFolder()}>Add a music folder</button>
        <p class="hint">…or drop files and folders anywhere in this window.</p>
      {/if}
    </div>
  {:else}
    <TrackTable
      {tracks}
      sortKey={library.sortKey}
      sortAsc={library.sortAsc}
      onSort={setSort}
      onActivate={(i) => playContext(tracks, i)}
    />
  {/if}
</div>

<style>
  .library {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  header {
    display: flex;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
  }
  .search {
    flex: 1;
    padding: 0.5rem 0.8rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }
  .search:focus {
    outline: none;
    border-color: var(--accent);
  }
  .scan-status {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.25rem 1rem 0.75rem;
  }
  progress {
    accent-color: var(--accent);
  }
  .scan-text {
    font-size: 0.8rem;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    color: var(--text-dim);
  }
  .empty-icon {
    font-size: 3rem;
  }
  .hint {
    font-size: 0.8rem;
  }
</style>
