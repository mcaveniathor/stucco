import { defineConfig, devices } from "@playwright/test";
import { fileURLToPath } from "node:url";
import { randomUUID } from "node:crypto";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const database = process.env.STUCCO_E2E_DATABASE
  ?? join(tmpdir(), `stucco-orders-browser-${randomUUID()}`, "orders.redb");
process.env.STUCCO_E2E_DATABASE = database;
// A copied runtime can be exercised without falling back to workspace assets.
const bundle = process.env.STUCCO_E2E_BUNDLE
  ? resolve(process.env.STUCCO_E2E_BUNDLE) : undefined;
if (bundle) process.env.STUCCO_E2E_BUNDLE = bundle;

// Build first from the repository root:
// cargo leptos build -p orders-fullstack --cargo-frozen
// Browsers share one disposable database and accounts, so keep tests serial.
export default defineConfig({
  testDir: "fullstack",
  outputDir: "test-results/fullstack",
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  use: {
    baseURL: "http://127.0.0.1:4183",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
  webServer: {
    command: "node browser/fullstack/server.mjs",
    cwd: fileURLToPath(new URL("../", import.meta.url)),
    env: {
      LEPTOS_OUTPUT_NAME: "orders-fullstack",
      LEPTOS_SITE_ROOT: bundle ? join(bundle, "site") : "target/orders-fullstack-site",
      LEPTOS_SITE_PKG_DIR: "pkg",
      LEPTOS_SITE_ADDR: "127.0.0.1:4183",
      ORDERS_ORIGIN: "http://127.0.0.1:4183",
      STUCCO_SECRET_KEY: "0".repeat(63) + "1",
      STUCCO_E2E_DATABASE: database,
      ORDERS_DATABASE: database,
      LEPTOS_ENV: bundle ? "PROD" : "DEV",
      LEPTOS_HASH_FILES: "false",
    },
    url: "http://127.0.0.1:4183/login",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
