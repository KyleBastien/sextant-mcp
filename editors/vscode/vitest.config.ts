import { defineConfig } from "vitest/config";
import * as path from "node:path";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    // Production sources double as test files (in-source testing). The
    // `typeof describe === "function"` guard in each file ensures the
    // describe blocks are no-ops outside vitest.
    include: ["src/**/*.ts", "test/lc-mock.ts"],
  },
  resolve: {
    alias: {
      vscode: path.resolve(__dirname, "test/vscode-mock.ts"),
      "vscode-languageclient/node": path.resolve(__dirname, "test/lc-mock.ts"),
    },
  },
});
