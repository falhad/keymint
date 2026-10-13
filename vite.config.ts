import { resolve } from "node:path";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// Two entry pages: the palette (index.html) and the lazily-created settings window.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**", "**/crates/**"] } },
  build: {
    target: "safari15",
    rollupOptions: {
      input: { main: resolve(__dirname, "index.html"), settings: resolve(__dirname, "settings.html") },
    },
  },
});
