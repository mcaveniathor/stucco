import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "visual",
  testMatch: "fonts.spec.ts",
  workers: 1,
  use: { browserName: "chromium", baseURL: "http://localhost:4184" },
  webServer: {
    command: "cargo run -q -p gallery --features atelier-fonts -- ../target/gallery-fonts && node serve.mjs ../target/gallery-fonts 4184",
    url: "http://localhost:4184/showcase-fonts.html",
    reuseExistingServer: false,
  },
});
