import { defineConfig } from "@playwright/test";
import base from "./playwright.config";

// Run workflow checks with just the static gallery; no database or WASM build.
export default defineConfig({
  ...base,
  testMatch: "workflows.spec.ts",
  webServer: Array.isArray(base.webServer) ? [base.webServer[1]] : base.webServer,
});
