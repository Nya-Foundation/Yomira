import { expect, test } from "@playwright/test";

test("reader resumes, returns to its gallery, and honors an explicit first page", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("exviewer.reader.pos.1", "9"));
  await page.goto("/read/1/abc");
  await expect(page.getByRole("slider", { name: "Page", exact: true })).toHaveValue("9");
  await page.getByRole("button", { name: "Go back" }).click();
  await expect(page).toHaveURL(/\/gallery\/1\/abc$/);
  await expect(page.getByRole("button", { name: "Continue at page 10" })).toBeVisible();
  await page.getByRole("button", { name: "Read from beginning" }).click();
  await expect(page.getByRole("slider", { name: "Page", exact: true })).toHaveValue("0");
  await page.getByRole("button", { name: "Hide reader controls" }).click();
  await expect(page.getByRole("slider", { name: "Page", exact: true })).not.toBeVisible();
  await page.getByRole("button", { name: "Show reader controls" }).click();
  await expect(page.getByRole("slider", { name: "Page", exact: true })).toBeVisible();
});

test("continuous reader restores its saved page without counting overscan as visible pages", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("exviewer.reader.pos.1", "12");
    localStorage.setItem("exviewer.settings", JSON.stringify({ readerMode: "continuous" }));
  });
  await page.goto("/read/1/abc");
  const slider = page.getByRole("slider", { name: "Page", exact: true });
  await expect(slider).toHaveValue("12");
  const scroll = page.locator("[data-reader-zoom]");
  const restoredPage = () => scroll.evaluate((el) => {
    const pageWidth = Math.max(0, Math.min(el.clientWidth - 24, 900));
    return Math.round(el.scrollTop / (pageWidth * 1.41 + 24));
  });
  await expect.poll(restoredPage).toBe(12);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect.poll(restoredPage).toBe(12);
  await expect(slider).toHaveValue("12");
  await page.setViewportSize({ width: 1440, height: 900 });
  await expect.poll(restoredPage).toBe(12);
  await expect(slider).toHaveValue("12");
  await scroll.evaluate((el) => { el.scrollTop = 0; });
  await expect(slider).toHaveValue("0");
});

// With the controls hidden, the only way back was a 44px button in the corner;
// the middle of the page did nothing. Paging edges and swipes must be
// unaffected by the tap that now toggles them.
test("a still tap in the middle toggles the controls, while edges and swipes still page", async ({ page }) => {
  await page.goto("/read/1/abc");
  const slider = page.getByRole("slider", { name: "Page", exact: true });
  await expect(slider).toHaveValue("0");

  const box = (await page.locator("[data-reader-surface]").boundingBox())!;
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;

  await page.mouse.click(cx, cy);
  await expect(slider).toBeHidden();
  await page.mouse.click(cx, cy);
  await expect(slider).toBeVisible();

  // right edge zone: pages, and leaves the controls alone
  await page.mouse.click(box.x + box.width * 0.93, cy);
  await expect(slider).toHaveValue("1");
  await expect(slider).toBeVisible();

  // a drag past the swipe threshold pages instead of toggling
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx - 120, cy, { steps: 10 });
  await page.mouse.up();
  await expect(slider).toHaveValue("2");
  await expect(slider).toBeVisible();
});
