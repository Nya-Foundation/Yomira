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
