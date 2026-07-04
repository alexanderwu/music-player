<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { formatDuration } from "../format";
  import {
    currentTrack,
    cycleRepeat,
    next,
    player,
    prev,
    seek,
    setVolume,
    togglePlay,
    toggleShuffle,
  } from "../state/player.svelte";

  let track = $derived(currentTrack());

  // While the user drags the slider we show the drag position, and only
  // seek on release — otherwise timeupdate fights the thumb.
  let dragging = $state(false);
  let dragValue = $state(0);
  let sliderValue = $derived(dragging ? dragValue : player.position);
</script>

<footer class="player-bar">
  <div class="now">
    {#if track}
      {#if track.coverPath}
        <img class="art" src={convertFileSrc(track.coverPath)} alt="" />
      {:else}
        <div class="art placeholder">♪</div>
      {/if}
      <div class="labels">
        <span class="title" title={track.title}>{track.title}</span>
        <span class="artist">{track.artist ?? "Unknown artist"}</span>
      </div>
    {:else}
      <div class="art placeholder">♪</div>
      <div class="labels">
        <span class="artist">Nothing playing</span>
      </div>
    {/if}
  </div>

  <div class="center">
    <div class="transport">
      <button
        class="mode"
        class:on={player.shuffle}
        title="Shuffle"
        onclick={toggleShuffle}>⤨</button
      >
      <button title="Previous" onclick={prev}>⏮</button>
      <button class="play" title="Play/Pause" onclick={togglePlay}>
        {player.isPlaying ? "⏸" : "▶"}
      </button>
      <button title="Next" onclick={() => next()}>⏭</button>
      <button
        class="mode"
        class:on={player.repeat !== "off"}
        title="Repeat: {player.repeat}"
        onclick={cycleRepeat}
      >
        {player.repeat === "one" ? "🔂" : "🔁"}
      </button>
    </div>
    <div class="seek">
      <span class="time">{formatDuration(sliderValue)}</span>
      <input
        type="range"
        min="0"
        max={player.duration || 1}
        step="0.1"
        value={sliderValue}
        disabled={!track}
        oninput={(e) => {
          dragging = true;
          dragValue = e.currentTarget.valueAsNumber;
        }}
        onchange={(e) => {
          seek(e.currentTarget.valueAsNumber);
          dragging = false;
        }}
      />
      <span class="time">{formatDuration(player.duration)}</span>
    </div>
  </div>

  <div class="volume">
    <span title="Volume">{player.volume === 0 ? "🔇" : "🔊"}</span>
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={player.volume}
      oninput={(e) => setVolume(e.currentTarget.valueAsNumber)}
    />
  </div>
</footer>

<style>
  .player-bar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 1rem;
    padding: 0.6rem 1rem;
    background: var(--surface);
    border-top: 1px solid var(--border);
  }
  .now {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    min-width: 0;
  }
  .art {
    width: 48px;
    height: 48px;
    border-radius: 6px;
    object-fit: cover;
    flex-shrink: 0;
  }
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--hover);
    color: var(--text-dim);
    font-size: 1.2rem;
  }
  .labels {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .artist {
    font-size: 0.8rem;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    width: min(46vw, 560px);
  }
  .transport {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .transport button {
    background: none;
    border: none;
    color: var(--text);
    font-size: 1.05rem;
    cursor: pointer;
    padding: 0.25rem 0.45rem;
    border-radius: 6px;
    line-height: 1;
  }
  .transport button:hover {
    background: var(--hover);
  }
  .transport .play {
    font-size: 1.5rem;
  }
  .transport .mode {
    font-size: 0.9rem;
    color: var(--text-dim);
  }
  .transport .mode.on {
    color: var(--accent);
  }
  .seek {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
  }
  .seek input {
    flex: 1;
    accent-color: var(--accent);
  }
  .time {
    font-size: 0.72rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
    min-width: 2.8em;
  }
  .volume {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-self: end;
  }
  .volume input {
    width: 110px;
    accent-color: var(--accent);
  }
</style>
