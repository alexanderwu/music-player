<script lang="ts">
  import { ui, showLibrary } from "../state/ui.svelte";
  import {
    createPlaylist,
    deletePlaylist,
    openPlaylist,
    playlists,
    renamePlaylist,
  } from "../state/playlists.svelte";

  let creating = $state(false);
  let newName = $state("");
  let renamingId: number | null = $state(null);
  let renameText = $state("");

  async function submitNew() {
    const name = newName.trim();
    creating = false;
    newName = "";
    if (!name) return;
    const playlist = await createPlaylist(name);
    ui.view = "playlist";
    await openPlaylist(playlist.id);
  }

  function startRename(id: number, current: string) {
    renamingId = id;
    renameText = current;
  }

  async function submitRename() {
    if (renamingId != null && renameText.trim()) {
      await renamePlaylist(renamingId, renameText.trim());
    }
    renamingId = null;
  }

  async function select(id: number) {
    ui.view = "playlist";
    await openPlaylist(id);
  }

  async function remove(id: number, name: string) {
    if (confirm(`Delete playlist “${name}”?`)) {
      await deletePlaylist(id);
      if (ui.view === "playlist" && playlists.activeId == null) showLibrary();
    }
  }
</script>

<nav class="sidebar">
  <button
    class="nav-item"
    class:active={ui.view === "library"}
    onclick={showLibrary}
  >
    ♫ Library
  </button>

  <div class="section">
    <span>Playlists</span>
    <button class="icon-btn" title="New playlist" onclick={() => (creating = true)}>＋</button>
  </div>

  {#if creating}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="inline-input"
      autofocus
      placeholder="Playlist name"
      bind:value={newName}
      onkeydown={(e) => {
        if (e.key === "Enter") void submitNew();
        if (e.key === "Escape") creating = false;
      }}
      onblur={submitNew}
    />
  {/if}

  {#each playlists.all as playlist (playlist.id)}
    {#if renamingId === playlist.id}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="inline-input"
        autofocus
        bind:value={renameText}
        onkeydown={(e) => {
          if (e.key === "Enter") void submitRename();
          if (e.key === "Escape") renamingId = null;
        }}
        onblur={submitRename}
      />
    {:else}
      <div
        class="nav-item playlist"
        class:active={ui.view === "playlist" && playlists.activeId === playlist.id}
        role="button"
        tabindex="0"
        onclick={() => select(playlist.id)}
        onkeydown={(e) => e.key === "Enter" && select(playlist.id)}
        ondblclick={() => startRename(playlist.id, playlist.name)}
      >
        <span class="name">{playlist.name}</span>
        <span class="count">{playlist.trackCount}</span>
        <button
          class="icon-btn delete"
          title="Delete playlist"
          onclick={(e) => {
            e.stopPropagation();
            void remove(playlist.id, playlist.name);
          }}>✕</button
        >
      </div>
    {/if}
  {/each}

  <div class="spacer"></div>
  <button
    class="nav-item"
    class:active={ui.view === "settings"}
    onclick={() => (ui.view = "settings")}
  >
    ⚙ Settings
  </button>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0.75rem 0.5rem;
    background: var(--surface);
    border-right: 1px solid var(--border);
    overflow-y: auto;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: var(--text);
    font: inherit;
    padding: 0.45rem 0.6rem;
    border-radius: 6px;
    cursor: pointer;
  }
  .nav-item:hover {
    background: var(--hover);
  }
  .nav-item.active {
    background: var(--active);
    color: var(--accent);
  }
  .section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 1rem;
    padding: 0.25rem 0.6rem;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
  }
  .icon-btn {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    border-radius: 4px;
    padding: 0 0.3rem;
  }
  .icon-btn:hover {
    color: var(--text);
    background: var(--hover);
  }
  .playlist .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .playlist .count {
    font-size: 0.7rem;
    color: var(--text-dim);
  }
  .playlist .delete {
    visibility: hidden;
  }
  .playlist:hover .delete {
    visibility: visible;
  }
  .inline-input {
    margin: 0 0.35rem;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }
  .spacer {
    flex: 1;
  }
</style>
