import path from "node:path";

import tailwindcss from "@tailwindcss/vite";
import solid from "vite-plugin-solid";
import { defineConfig } from "vitest/config";

import type { Plugin } from "vite";

function parentProcessWatchdog(): Plugin | null {
  const parentProcessId = Number(process.env.CURSOR_OPTIMIZER_PID);
  if (!Number.isFinite(parentProcessId) || parentProcessId <= 0) {
    return null;
  }
  return {
    name: "parent-process-watchdog",
    configureServer() {
      const timer = setInterval(() => {
        try {
          process.kill(parentProcessId, 0);
        } catch {
          clearInterval(timer);
          process.exit(0);
        }
      }, 1000);
      timer.unref();
    },
  };
}

export default defineConfig({
  base: "./",
  plugins: [solid(), tailwindcss(), parentProcessWatchdog()],
  resolve: {
    alias: {
      "@cursor-optimizer/design-system": path.resolve(
        import.meta.dirname,
        "../../../../libraries/design-system",
      ),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "../../../../distributions/web",
    emptyOutDir: true,
    target: "esnext",
  },
  test: {
    environment: "node",
    include: ["sources/**/*.test.ts"],
  },
});
