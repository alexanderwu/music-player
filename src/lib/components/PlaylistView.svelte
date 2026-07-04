<script lang="ts">
  import TrackTable from "./TrackTable.svelte";
  import {
    activePlaylist,
    playlists,
    removeAt,
    reorderActive,
  } from "../state/playlists.svelte";
  import { playContext } from "../state/player.svelte";

  let playlist = $derived(activePlaylist());
</script>

<div class="playlist-view">
  {#if playlist}
    <header>
      <h2>{playlist.name}</h2>
      <span class="meta">{playlist.trackCount} track{playlist.trackCount === 1 ? "" : "s"}</span>
    </header>

    {#if playlists.activeTracks.length === 0}
      <div class="empty">
        <p>This playlist is empty.</p>
        <p class="hint">Right-click a track in the library to add it here.</p>
      </div>
    {:else}
      <TrackTable
        tracks={playlists.activeTracks}
        onActivate={(i) => playContext(playlists.activeTracks, i)}
        reorderable
        onReorder={(from, to) => void reorderActive(from, to)}
        onRemove={(i) => void removeAt(i)}
      />
    {/if}
  {/if}
</div>

<style>
  .playlist-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    padding: 0.9rem 1rem 0.5rem;
  }
  h2 {
    margin: 0;
    font-size: 1.2rem;
  }
  .meta {
    color: var(--text-dim);
    font-size: 0.8rem;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    color: var(--text-dim);
  }
  .hint {
    font-size: 0.8rem;
  }
</style>
