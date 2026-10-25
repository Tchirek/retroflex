import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// 開発の　門の向こうに　中枢あり
const hub = process.env.RETROFLEX_DEV_HUB ?? "http://127.0.0.1:7746";

export default defineConfig({
  plugins: [svelte()],
  define: { __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "0.0.0") },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: process.env.TAURI_DEV_HOST || false,
    proxy: { "/api": { target: hub, ws: true, changeOrigin: true } },
  },
  build: { target: "es2022", chunkSizeWarningLimit: 800 },
});
