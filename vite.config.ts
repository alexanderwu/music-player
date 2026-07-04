import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri points the webview at this dev server during `tauri dev`;
// in production the built files in `dist/` are embedded in the binary.
export default defineConfig({
  plugins: [svelte()],

  // Don't clear the terminal — it would hide Rust compiler output.
  clearScreen: false,

  server: {
    // Tauri expects a fixed port (see build.devUrl in tauri.conf.json).
    port: 1420,
    strictPort: true,
    watch: {
      // Don't let Vite watch the Rust side; cargo output would trigger reloads.
      ignored: ["**/src-tauri/**"],
    },
  },
});
