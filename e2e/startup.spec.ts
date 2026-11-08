import { expect, test } from "@playwright/test";
import { installFixture } from "../scripts/profiling/fixture.mjs";

// The first paint waited on two IPC calls so the saved display scale would not
// flash. A cold start can drop the response to the first call, leaving those
// promises pending for good — and the app then rendered nothing but the dark
// canvas, a black screen that only a reload cleared. The gate has to give up.
test("a lost startup IPC response does not leave a black screen", async ({ page }) => {
  await page.addInitScript(installFixture);
  await page.addInitScript(() => {
    const w = window as any;
    const original = w.__TAURI_INTERNALS__.invoke;
    const stuck = new Set(["image_base_url", "plugin:store|load"]);
    w.__TAURI_INTERNALS__.invoke = (command: string, args: unknown) =>
      stuck.has(command) ? new Promise(() => {}) : original(command, args);
  });

  await page.goto("/");
  await expect(page.getByRole("link", { name: "Home" }).first()).toBeVisible({ timeout: 8000 });
  expect(await page.evaluate(() => document.getElementById("root")!.childElementCount)).toBeGreaterThan(0);
});
