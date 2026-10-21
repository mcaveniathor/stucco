import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

// Opt-in captures; keep them out of the behavioral suites.
export default defineConfig({
  ...base,
  testDir: "visual",
  testIgnore: "fonts.spec.ts",
  webServer: Array.isArray(base.webServer) ? base.webServer.map((server, index) => ({
    ...server,
    reuseExistingServer: false,
    ...(index === 0 ? {
      command: "rm -f ../target/orders-visual.redb && cargo run -q -p orders",
      env: { ...server.env, ORDERS_DB: "../target/orders-visual.redb" },
    } : {}),
  })) : base.webServer,
  fullyParallel: false,
  workers: 1,
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
