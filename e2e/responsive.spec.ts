import { expect, test } from "@playwright/test";

test("mobile navigation exposes secondary destinations and preserves modal focus", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const more = page.getByRole("button", { name: "More", exact: true });
  await more.click();
  const menu = page.getByRole("dialog", { name: "More", exact: true });
  await expect(menu).toBeVisible();
  await page.keyboard.press("Shift+Tab");
  expect(await menu.evaluate((el) => el.contains(document.activeElement))).toBe(true);
  await page.keyboard.press("Escape");
  await expect(menu).not.toBeVisible();
  await expect(more).toBeFocused();
  await more.click();
  await menu.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(page).toHaveURL(/\/settings$/);
  await expect(menu).not.toBeVisible();
  await page.getByRole("tab", { name: "Network", exact: true }).click();
  const apply = page.getByRole("button", { name: "Apply", exact: true });
  await apply.scrollIntoViewIfNeeded();
  const bounds = await apply.boundingBox();
  const nav = await page.getByRole("navigation", { name: "Primary", exact: true }).boundingBox();
  expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(nav!.y);
});

test("search input keeps focus while a modal value changes", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Jump to page", exact: true }).click();
  const input = page.locator("#jump-page");
  await input.fill("2");
  await expect(input).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("button", { name: "Jump to page", exact: true })).toBeFocused();
});

test("gallery rows resize without overlapping at 200 percent display scale", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("exviewer.settings", JSON.stringify({ displayScale: 2, theme: "light" }));
  });
  await page.setViewportSize({ width: 1024, height: 900 });
  await page.goto("/");
  await expect(page.locator("main a[href^='/gallery/']").first()).toBeVisible();
  const rows = page.locator("main [data-index]");
  await expect.poll(() => rows.count()).toBeGreaterThan(1);
  const first = await rows.nth(0).boundingBox();
  const second = await rows.nth(1).boundingBox();
  expect(second!.y).toBeGreaterThanOrEqual(first!.y + first!.height - 1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test("narrow screens keep settings, details and favorites within the viewport", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 640 });
  for (const route of ["/", "/settings", "/favorites", "/downloads", "/history", "/gallery/1/abc"]) {
    await page.goto(route);
    await expect(page.locator("header h1")).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), route).toBe(true);
  }
});

test("browsing destinations retain independent searches", async ({ page }) => {
  await page.goto("/");
  const search = page.getByRole("combobox", { name: "Search galleries", exact: true });
  await search.fill("language:english");
  await expect.poll(() => page.evaluate(() => JSON.parse(sessionStorage.getItem("exviewer.liststate.home")!).params.search)).toBe("language:english");
  await page.getByRole("link", { name: "Popular", exact: true }).click();
  await expect(search).toHaveValue("");
  await search.fill("language:japanese");
  await expect.poll(() => page.evaluate(() => JSON.parse(sessionStorage.getItem("exviewer.liststate.popular")!).params.search)).toBe("language:japanese");
  await page.getByRole("link", { name: "Home", exact: true }).click();
  await expect(search).toHaveValue("language:english");
});

test("small primary buttons retain readable text in both themes", async ({ page }) => {
  await page.goto("/settings");
  for (const theme of ["light", "dark"]) {
    await page.getByRole("tab", { name: "General", exact: true }).click();
    await page.getByRole("combobox", { name: "Theme", exact: true }).selectOption(theme);
    await page.getByRole("tab", { name: "Network", exact: true }).click();
    const button = page.getByRole("button", { name: "Apply", exact: true });
    await expect.poll(() => button.evaluate((element) => {
      const style = getComputedStyle(element);
      const luminance = (color: string) => {
        const channels = color.match(/[\d.]+/g)!.slice(0, 3).map(Number).map((channel) => {
          const value = channel / 255;
          return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
        });
        return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
      };
      const a = luminance(style.color);
      const b = luminance(style.backgroundColor);
      return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
    })).toBeGreaterThanOrEqual(4.5);
  }
});

test("feedback remains visible when another modal opens", async ({ page }) => {
  await page.goto("/downloads");
  await page.getByRole("button", { name: "New label", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("textbox").fill("Reading next");
  await dialog.getByRole("button", { name: "Add", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.locator('[aria-live="polite"]')).toContainText("Label created");
  await page.getByRole("button", { name: "New label", exact: true }).click();
  await expect(dialog.locator('[aria-live="polite"]')).toContainText("Label created");
});
