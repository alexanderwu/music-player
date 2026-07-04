<script lang="ts">
  // Hand-rolled virtualized list: only the rows in (and just around) the
  // viewport exist in the DOM, so a 10k-track library stays smooth.
  // Spacer padding above/below keeps the scrollbar honest.
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { SortKey, Track } from "../types";
  import { formatDuration } from "../format";
  import { currentTrack, player } from "../state/player.svelte";
  import { addToPlaylist, createPlaylist, playlists } from "../state/playlists.svelte";

  interface Props {
    tracks: Track[];
    onActivate: (index: number) => void;
    /** Column-header sorting (library view only). */
    sortKey?: SortKey;
    sortAsc?: boolean;
    onSort?: (key: SortKey) => void;
    /** Playlist view: enable drag-to-reorder and per-row remove. */
    reorderable?: boolean;
    onReorder?: (from: number, to: number) => void;
    onRemove?: (index: number) => void;
  }

  let {
    tracks,
    onActivate,
    sortKey,
    sortAsc,
    onSort,
    reorderable = false,
    onReorder,
    onRemove,
  }: Props = $props();

  const ROW_HEIGHT = 44;
  const OVERSCAN = 8;

  let viewport: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewportHeight = $state(600);

  let start = $derived(
    Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN),
  );
  let end = $derived(
    Math.min(
      tracks.length,
      Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN,
    ),
  );
  let visible = $derived(tracks.slice(start, end));

  function onScroll() {
    if (viewport) scrollTop = viewport.scrollTop;
  }

  // ---- Context menu (right-click → add to playlist) ----
  let menu = $state<{ x: number; y: number; track: Track } | null>(null);

  function openMenu(e: MouseEvent, track: Track) {
    e.preventDefault();
    menu = { x: e.clientX, y: e.clientY, track };
  }

  async function addTo(playlistId: number) {
    if (menu) await addToPlaylist(playlistId, [menu.track.id]);
    menu = null;
  }

  async function addToNew() {
    if (!menu) return;
    const track = menu.track;
    menu = null;
    // No text-input dialogs in webviews — create with a default name;
    // double-click in the sidebar renames.
    const playlist = await createPlaylist(`New playlist ${playlists.all.length + 1}`);
    await addToPlaylist(playlist.id, [track.id]);
  }

  // ---- Drag to reorder (playlist view) ----
  let dragFrom: number | null = $state(null);
  let dragOver: number | null = $state(null);

  function headerClick(key: SortKey) {
    onSort?.(key);
  }

  const columns: { key: SortKey; label: string; class: string }[] = [
    { key: "title", label: "Title", class: "col-title" },
    { key: "artist", label: "Artist", class: "col-artist" },
    { key: "album", label: "Album", class: "col-album" },
    { key: "durationSecs", label: "⏱", class: "col-duration" },
  ];
</script>

<svelte:window onclick={() => (menu = null)} />

<div class="table">
  <div class="header row">
    <span class="col-art"></span>
    {#each columns as col (col.key)}
      {#if onSort}
        <button class={`col ${col.class}`} onclick={() => headerClick(col.key)}>
          {col.label}
          {#if sortKey === col.key}<span class="sort-arrow">{sortAsc ? "▲" : "▼"}</span>{/if}
        </button>
      {:else}
        <span class={`col ${col.class}`}>{col.label}</span>
      {/if}
    {/each}
    {#if onRemove}<span class="col-remove"></span>{/if}
  </div>

  <div
    class="viewport"
    bind:this={viewport}
    bind:clientHeight={viewportHeight}
    onscroll={onScroll}
  >
    <div style:padding-top="{start * ROW_HEIGHT}px" style:padding-bottom="{(tracks.length - end) * ROW_HEIGHT}px">
      {#each visible as track, i (start + i)}
        {@const index = start + i}
        {@const isCurrent = currentTrack()?.id === track.id}
        {@const hasError = player.errorTrackIds.has(track.id)}
        <div
          class="row track-row"
          class:current={isCurrent}
          class:error={hasError}
          class:drag-over={dragOver === index}
          role="button"
          tabindex="0"
          style:height="{ROW_HEIGHT}px"
          ondblclick={() => onActivate(index)}
          onkeydown={(e) => e.key === "Enter" && onActivate(index)}
          oncontextmenu={(e) => openMenu(e, track)}
          draggable={reorderable}
          ondragstart={(e) => {
            dragFrom = index;
            e.dataTransfer?.setData("text/plain", String(index));
          }}
          ondragover={(e) => {
            if (reorderable && dragFrom != null) {
              e.preventDefault();
              dragOver = index;
            }
          }}
          ondrop={(e) => {
            e.preventDefault();
            if (dragFrom != null && dragFrom !== index) onReorder?.(dragFrom, index);
            dragFrom = null;
            dragOver = null;
          }}
          ondragend={() => {
            dragFrom = null;
            dragOver = null;
          }}
        >
          <span class="col-art">
            {#if track.coverPath}
              <img src={convertFileSrc(track.coverPath)} alt="" loading="lazy" />
            {:else}
              <span class="art-placeholder">♪</span>
            {/if}
          </span>
          <span class="col col-title" title={track.path}>
            {#if isCurrent}<span class="now-playing">{player.isPlaying ? "▶" : "❚❚"}</span>{/if}
            {track.title}
            {#if hasError}<span class="badge" title="Can't play this file — missing or unsupported format">unplayable</span>{/if}
          </span>
          <span class="col col-artist">{track.artist ?? "—"}</span>
          <span class="col col-album">{track.album ?? "—"}</span>
          <span class="col col-duration">{formatDuration(track.durationSecs)}</span>
          {#if onRemove}
            <button
              class="col-remove remove-btn"
              title="Remove from playlist"
              onclick={(e) => {
                e.stopPropagation();
                onRemove?.(index);
              }}>✕</button
            >
          {/if}
        </div>
      {/each}
    </div>
  </div>
</div>

{#if menu}
  <div class="context-menu" style:left="{menu.x}px" style:top="{menu.y}px">
    <div class="menu-label">Add to playlist</div>
    {#each playlists.all as playlist (playlist.id)}
      <button onclick={() => addTo(playlist.id)}>{playlist.name}</button>
    {/each}
    <button class="menu-new" onclick={addToNew}>New playlist…</button>
  </div>
{/if}

<style>
  .table {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .viewport {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }
  .row {
    display: grid;
    grid-template-columns: 44px minmax(0, 2fr) minmax(0, 1.2fr) minmax(0, 1.2fr) 56px auto;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.75rem;
  }
  .header {
    border-bottom: 1px solid var(--border);
    padding-top: 0.4rem;
    padding-bottom: 0.4rem;
    font-size: 0.8rem;
    color: var(--text-dim);
    user-select: none;
  }
  .header button {
    background: none;
    border: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    padding: 0;
  }
  .header button:hover {
    color: var(--text);
  }
  .sort-arrow {
    font-size: 0.65rem;
    margin-left: 0.25rem;
  }
  .track-row {
    border-radius: 6px;
    cursor: default;
  }
  .track-row:hover {
    background: var(--hover);
  }
  .track-row.current {
    color: var(--accent);
  }
  .track-row.error {
    opacity: 0.55;
  }
  .track-row.drag-over {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .col {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .col-art {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .col-art img {
    width: 32px;
    height: 32px;
    object-fit: cover;
    border-radius: 4px;
  }
  .art-placeholder {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--hover);
    border-radius: 4px;
    color: var(--text-dim);
  }
  .col-duration {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-dim);
  }
  .now-playing {
    margin-right: 0.4rem;
    font-size: 0.7rem;
  }
  .badge {
    margin-left: 0.5rem;
    font-size: 0.65rem;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: var(--danger-bg);
    color: var(--danger);
  }
  .remove-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    padding: 0.25rem 0.4rem;
    border-radius: 4px;
    visibility: hidden;
  }
  .track-row:hover .remove-btn {
    visibility: visible;
  }
  .remove-btn:hover {
    color: var(--danger);
    background: var(--danger-bg);
  }
  .context-menu {
    position: fixed;
    z-index: 100;
    min-width: 180px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.25);
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
  }
  .context-menu .menu-label {
    font-size: 0.7rem;
    color: var(--text-dim);
    padding: 0.3rem 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .context-menu button {
    background: none;
    border: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    padding: 0.4rem 0.6rem;
    border-radius: 5px;
    cursor: pointer;
  }
  .context-menu button:hover {
    background: var(--hover);
  }
  .menu-new {
    border-top: 1px solid var(--border);
    margin-top: 0.25rem;
  }
</style>
