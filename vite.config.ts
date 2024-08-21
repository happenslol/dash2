import { resolve } from "node:path"
import { defineConfig } from "vite"
import { visualizer } from "rollup-plugin-visualizer"
import solidPlugin from "vite-plugin-solid"

export default defineConfig(async () => ({
  plugins: [solidPlugin(), visualizer()],

  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        login: resolve(__dirname, "login.html"),
      },
    },
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
}))
