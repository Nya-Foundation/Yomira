import { expect, test } from "@playwright/test";

const continuous = () =>
  localStorage.setItem("exviewer.settings", JSON.stringify({ readerMode: "continuous" }));

// Space and PageDown reached the page-level handler, which moves the page
// index — so in the one mode built for reading across the seam, they jumped a
// whole page instead of scrolling.
test("continuous mode scrolls with the keyboard instead of paging", async ({ page }) => {
  await page.addInitScript(continuous);
  await page.goto("/read/1/abc");
  const scroll = page.locator("[data-reader-zoom]");
  await expect(scroll).toBeVisible();
  const top = () => scroll.evaluate((el) => el.scrollTop);
  const viewport = await scroll.evaluate((el) => el.clientHeight);

  await page.keyboard.press("ArrowDown");
  await expect.poll(top).toBe(120);

  await page.keyboard.press("ArrowUp");
  await expect.poll(top).toBe(0);

  // a screenful, not a page
  await page.keyboard.press("PageDown");
  await expect.poll(top).toBeCloseTo(viewport * 0.9, 0);

  await page.keyboard.press("Space");
  await expect.poll(top).toBeCloseTo(viewport * 1.8, 0);

  await page.keyboard.press("Shift+Space");
  await expect.poll(top).toBeCloseTo(viewport * 0.9, 0);
});

test("reaching the last page offers somewhere to go", async ({ page }) => {
  await page.goto("/read/1/abc");
  const slider = page.getByRole("slider", { name: "Page", exact: true });
  await expect(slider).toHaveValue("0");
  const end = page.getByRole("group", { name: "End of gallery" });
  await expect(end).toBeHidden();

  await page.keyboard.press("End");
  const last = await slider.inputValue();
  expect(Number(last)).toBeGreaterThan(0);
  // at the last page, nothing told you so: forward was simply dead
  await expect(end).toBeHidden();

  await page.keyboard.press("j");
  await expect(end).toBeVisible();
  await expect(slider).toHaveValue(last);

  await end.getByRole("button", { name: "Read again" }).click();
  await expect(slider).toHaveValue("0");
  await expect(end).toBeHidden();
});

test("fit-width fills the width in both modes", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.addInitScript(continuous);
  await page.goto("/read/1/abc");
  const strip = page.locator("[data-reader-zoom] > div");
  // the comfort cap holds the strip at 900 while fitting whole pages
  await expect.poll(() => strip.evaluate((el) => Math.round(el.getBoundingClientRect().width))).toBe(900);

  const fit = page.getByRole("button", { name: "Toggle page fit" });
  await fit.click();
  await expect(fit).toHaveAttribute("aria-pressed", "true");
  await expect
    .poll(() => strip.evaluate((el) => Math.round(el.getBoundingClientRect().width)))
    .toBeGreaterThan(1200);

  // paged mode gains a scroller so a page taller than the viewport is reachable
  await page.getByRole("button", { name: "Toggle reading mode" }).click();
  const scroller = page.locator("[data-page-scroll]");
  await expect(scroller).toBeVisible();
  const widths = await page.evaluate(() => {
    const box = document.querySelector("[data-page-scroll]")!;
    const img = box.querySelector("img");
    return { box: Math.round(box.clientWidth), img: img ? Math.round(img.getBoundingClientRect().width) : -1 };
  });
  expect(widths.img).toBe(widths.box);
});
