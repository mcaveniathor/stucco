import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  use: { baseURL: "http://localhost:4173" },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
  webServer: [
    {
      command: "cargo run -q -p orders",
      env: { PORT: "4181", ORDERS_DB: "../target/orders-browser.redb" },
      url: "http://localhost:4181/orders",
      reuseExistingServer: !process.env.CI,
      timeout: 300_000,
    },
    {
      command: "cargo run -q -p gallery -- ../target/gallery && node serve.mjs ../target/gallery 4173",
      url: "http://localhost:4173/index.html",
      reuseExistingServer: !process.env.CI,
      timeout: 300_000,
    },
    {
      command: "cargo run -q -p hello",
      env: { PORT: "4180" },
      url: "http://localhost:4180/",
      reuseExistingServer: !process.env.CI,
      timeout: 300_000,
    },
  ],
});
