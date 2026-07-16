import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Test-only config (separate from vite.config.ts, which builds the app).
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      provider: "v8",
      // The pure logic + IPC contract + store are measured. Presentational
      // React components (screens/, components/, App.tsx) and the bootstrap
      // entry are the UI/integration layer, exercised by behavioural tests
      // rather than gated on line coverage; types.ts has no runtime code.
      include: ["src/markdown.ts", "src/labels.ts", "src/ipc.ts", "src/store.tsx"],
      reporter: ["text", "json-summary"],
      // 100% is enforced on the pure logic modules.
      thresholds: {
        "src/markdown.ts": { statements: 100, branches: 100, functions: 100, lines: 100 },
        "src/labels.ts": { statements: 100, branches: 100, functions: 100, lines: 100 },
        "src/ipc.ts": { statements: 100, branches: 100, functions: 100, lines: 100 },
      },
    },
  },
});
