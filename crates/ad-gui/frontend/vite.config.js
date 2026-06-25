import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri serves this app; keep the dev server on a fixed port so
// tauri.conf.json's devUrl matches, and don't clear the terminal.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
