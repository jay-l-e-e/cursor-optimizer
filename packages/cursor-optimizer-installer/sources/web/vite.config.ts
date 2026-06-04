import path from "node:path";

import tailwindcss from "@tailwindcss/vite";
import solid from "vite-plugin-solid";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "./",
  plugins: [solid(), tailwindcss()],
  resolve: {
    alias: {
      "@cursor-optimizer/design-system": path.resolve(
        import.meta.dirname,
        "../../../../libraries/design-system",
      ),
    },
  },
  server: {
    port: 5174,
    strictPort: true,
  },
  build: {
    outDir: "../../../../distributions/web-installer",
    emptyOutDir: true,
    target: "esnext",
  },
  test: {
    environment: "node",
    include: ["sources/**/*.test.ts"],
  },
});
