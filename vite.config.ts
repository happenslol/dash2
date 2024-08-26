import { resolve } from "node:path"
import { defineConfig } from "vite"
import solid from "vite-plugin-solid"

const host = process.env.TAURI_DEV_HOST

export default defineConfig(async () => ({
  plugins: [solid()],

  build: {
    rollupOptions: {
      input: {
        login: resolve(__dirname, "./src/login/index.html"),
      },
    },
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}))
