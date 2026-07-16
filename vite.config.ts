import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// index.html and the React sources live in `src/`, so that is the Vite root.
// The production bundle is emitted to `../dist`, which Tauri serves as
// `frontendDist` (see src-tauri/tauri.conf.json).
export default defineConfig({
  root: "src",
  plugins: [react()],
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    minify: "esbuild",
    target: "es2021",
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
});
