import { expect, test } from "@playwright/test";

// The grid persists its offset per history entry. The save used to re-read
// scrollTop when it wrote, which happens after teardown has clamped the
// container to 0 — so the restore faithfully applied a near-zero offset
// however far the user had scrolled. Browsing a feed and opening a gallery is
// the most repeated journey in the app, so this guards it directly.
test("returning from a gallery restores your place in the feed", async ({ page }) => {
  await page.goto("/");
  const scroller = page.locator("[data-grid-scroll]");
  await expect(scroller).toBeVisible();
  // Rows are virtualized off a measured width, so wait for a real row rather
  // than the first anchor appearing.
  await page.locator("[data-index]").first().waitFor();

  const target = await scroller.evaluate((el) => {
    el.scrollTop = Math.min(600, el.scrollHeight - el.clientHeight);
    return el.scrollTop;
  });
  expect(target).toBeGreaterThan(100);
  // The grid records the offset itself; wait for that rather than racing it.
  await expect
    .poll(() => page.evaluate(() => {
      const hit = Object.entries(sessionStorage).find(([k]) => k.startsWith("exviewer.scroll"));
      return hit ? Number(hit[1]) : -1;
    }))
    .toBeGreaterThan(target - 60);

  // Click a card that is already on screen. Clicking the first card would make
  // Playwright scroll back to the top to reach it, which really does move the
  // user and would make this test assert its own artefact.
  const inView = page.locator('a[href^="/gallery/"]').last();
  await inView.scrollIntoViewIfNeeded();
  const before = await scroller.evaluate((el) => el.scrollTop);
  await inView.click();
  await expect(page).toHaveURL(/\/gallery\//);

  await page.goBack();
  await expect(page).toHaveURL(/\/$/);
  await page.locator('a[href^="/gallery/"]').first().waitFor();

  // Restoration lands once rows and width are measured, so poll rather than
  // assume it happened by the time the first card renders.
  await expect
    .poll(() => scroller.evaluate((el) => el.scrollTop), { timeout: 5000 })
    .toBeGreaterThan(before - 60);
});
