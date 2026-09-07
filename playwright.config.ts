import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  fullyParallel: true,
  workers: 2,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:1420",
    viewport: { width: 1440, height: 900 },
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      testIgnore: /tag-translation\.spec\.ts/,
      use: { ...devices["Desktop Chrome"] },
    },
    {
      // zh-CN locale project: only the tag-translation spec runs here. The
      // app derives its effective locale from navigator.language when no
      // language preference is persisted, so this exercises the zh-CN path.
      name: "chromium-zh",
      testMatch: /tag-translation\.spec\.ts/,
      use: { ...devices["Desktop Chrome"], locale: "zh-CN" },
    },
  ],
  webServer: {
    command: "bun run dev",
    port: 1420,
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
