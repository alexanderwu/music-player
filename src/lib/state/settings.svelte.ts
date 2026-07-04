// Settings live in tauri-plugin-store (a small JSON file) — the right tool
// for tiny key-value data, while the library lives in SQLite. Matching
// storage to data shape is the lesson (PLAN 3.5).

import { LazyStore } from "@tauri-apps/plugin-store";
import { player, setVolume } from "./player.svelte";

export type Theme = "system" | "light" | "dark";

const store = new LazyStore("settings.json");

export const settings = $state({
  theme: "system" as Theme,
  closeToTray: true,
});

function applyTheme(theme: Theme) {
  if (theme === "system") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", theme);
  }
}

/** Load persisted settings and start persisting changes. Call once on mount. */
export async function initSettings() {
  const [volume, theme, closeToTray] = await Promise.all([
    store.get<number>("volume"),
    store.get<Theme>("theme"),
    store.get<boolean>("closeToTray"),
  ]);
  if (typeof volume === "number") setVolume(volume);
  if (theme) settings.theme = theme;
  if (typeof closeToTray === "boolean") settings.closeToTray = closeToTray;
  applyTheme(settings.theme);

  // Persist on change. $effect.root because we're outside a component.
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect.root(() => {
    $effect(() => {
      // Read everything we persist so the effect tracks it.
      const snapshot = {
        volume: player.volume,
        theme: settings.theme,
        closeToTray: settings.closeToTray,
      };
      applyTheme(snapshot.theme);
      clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        void store.set("volume", snapshot.volume);
        void store.set("theme", snapshot.theme);
        void store.set("closeToTray", snapshot.closeToTray);
      }, 300);
    });
  });
}
