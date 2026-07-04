<script lang="ts">
  import { settings, type Theme } from "../state/settings.svelte";

  const themes: { value: Theme; label: string }[] = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];
</script>

<div class="settings">
  <h2>Settings</h2>

  <section>
    <h3>Appearance</h3>
    <label class="field">
      <span>Theme</span>
      <select bind:value={settings.theme}>
        {#each themes as t (t.value)}
          <option value={t.value}>{t.label}</option>
        {/each}
      </select>
    </label>
  </section>

  <section>
    <h3>Behavior</h3>
    <label class="field checkbox">
      <input type="checkbox" bind:checked={settings.closeToTray} />
      <span>
        Closing the window hides the app to the system tray
        <small>Use Quit in the tray menu to exit completely.</small>
      </span>
    </label>
  </section>
</div>

<style>
  .settings {
    padding: 1.25rem 1.5rem;
    max-width: 560px;
  }
  h2 {
    margin: 0 0 1rem;
    font-size: 1.2rem;
  }
  section {
    margin-bottom: 1.5rem;
  }
  h3 {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    margin: 0 0 0.6rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .field select {
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.35rem 0.5rem;
    font: inherit;
  }
  .checkbox {
    align-items: flex-start;
  }
  .checkbox input {
    margin-top: 0.2rem;
    accent-color: var(--accent);
  }
  .checkbox small {
    display: block;
    color: var(--text-dim);
  }
</style>
