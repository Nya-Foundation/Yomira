import { expect, test } from "@playwright/test";

// Browser fixtures exercise the UI; the Rust integration test separately
// downloads, reopens SQLite, and streams real files with no network source calls.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    if (localStorage.getItem("offline-test-seeded")) return;
    localStorage.setItem("offline-test-seeded", "1");
    localStorage.setItem("exviewer.downloads.seeded", "1");
    localStorage.setItem("exviewer.downloads", JSON.stringify([
      { gid: 1, token: "saved", state: "paused", label: null, finishedPages: 3, error: null, time: 1 },
    ]));
    localStorage.setItem("exviewer.reader.pos.1", "1");
  });
  // No remote hosts are available. The local Vite app can still reload.
  await page.route(/^https?:\/\/(?!localhost:1420(?:\/|$))/, (route) => route.abort());
});

test("Downloads opens saved pages, resumes after reload and returns to Downloads", async ({ page }) => {
  await page.goto("/downloads");
  await page.getByRole("link", { name: /^Read offline:/ }).click();
  await expect(page).toHaveURL(/\/read\/1\/saved\?source=download$/);
  await expect(page.getByRole("status").filter({ hasText: "Offline · 3 of" })).toBeVisible();
  const slider = page.getByRole("slider", { name: "Page", exact: true });
  await expect(slider).toHaveValue("1");
  await expect(page.getByRole("img", { name: "Page 2", exact: true })).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(slider).toHaveValue("2");
  await page.reload();
  await expect(slider).toHaveValue("2");
  await expect(page.getByRole("img", { name: "Page 3", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Go back", exact: true }).click();
  await expect(page).toHaveURL(/\/downloads$/);
});

test("missing pages stay offline and retry when saved files become available", async ({ page }) => {
  await page.goto("/read/1/saved?source=download&p=3");
  await expect(page.getByText("Saved page unavailable", { exact: true })).toBeVisible();
  await expect(page.getByRole("img")).toHaveCount(0);
  await page.evaluate(() => {
    const entries = JSON.parse(localStorage.getItem("exviewer.downloads")!);
    entries[0].finishedPages = 4;
    localStorage.setItem("exviewer.downloads", JSON.stringify(entries));
  });
  await page.getByRole("button", { name: "Retry", exact: true }).click();
  await expect(page.getByRole("img", { name: "Page 4", exact: true })).toBeVisible();
  await expect(page.getByRole("status")).toContainText("Offline · 4 of");
});

test("an unknown or removed download does not fall back to gallery loading", async ({ page }) => {
  await page.goto("/read/1/wrong-token?source=download");
  await expect(page.getByText(/This download is unavailable/)).toBeVisible();
  await expect(page.getByRole("slider")).toHaveCount(0);
  await page.getByRole("link", { name: "Back to Downloads" }).click();
  await expect(page).toHaveURL(/\/downloads$/);
});

test("continuous offline reading restores the saved position at a narrow width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/read/1/saved?source=download");
  await page.getByRole("button", { name: "Toggle reading mode" }).click();
  await expect(page.getByRole("slider", { name: "Page", exact: true })).toHaveValue("1");
  await expect(page.getByRole("img", { name: "Page 2", exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.reload();
  await expect(page.getByRole("slider", { name: "Page", exact: true })).toHaveValue("1");
});

test("an unreadable saved image exposes recovery instead of a broken image", async ({ page }) => {
  await page.goto("/downloads");
  await page.getByRole("link", { name: /^Read offline:/ }).click();
  const image = page.getByRole("img", { name: "Page 2", exact: true });
  await expect(image).toBeVisible();
  await image.evaluate((element: HTMLImageElement) => { element.src = "data:image/png;base64,AA=="; });
  await expect(page.getByText("Saved page unavailable", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Retry", exact: true })).toBeVisible();
});
