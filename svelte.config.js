import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  // Handles <script lang="ts"> blocks.
  preprocess: vitePreprocess(),
};
