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
  await page.getByRole("link", { name: "Watched", exact: true }).click();
  await expect(search).toHaveValue("");
  await search.fill("language:japanese");
  await expect.poll(() => page.evaluate(() => JSON.parse(sessionStorage.getItem("exviewer.liststate.watched")!).params.search)).toBe("language:japanese");
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

// The popular feed is a single fixed page: the backend discards keyword,
// categories, advanced search and even the page number for it. Showing those
// controls would show filters that silently do nothing.
test("popular offers no filter the feed cannot honour", async ({ page }) => {
  await page.goto("/popular");
  await expect(page.locator("[data-load-more]")).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Search galleries", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Category filters" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Advanced search" })).toHaveCount(0);

  // Home still has all three.
  await page.goto("/");
  await expect(page.getByRole("combobox", { name: "Search galleries", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Category filters" })).toBeVisible();
});

// Rotation changes the column count and the row height at once, so keeping the
// pixel offset pointed somewhere else in the list entirely — worse the further
// you had scrolled. The card you were looking at has to survive the turn.
// It cannot stay in the same cell (the columns themselves change), so the
// contract is that it stays in the top visible row.
test("rotating the tablet keeps your place in the grid", async ({ page }) => {
  await page.setViewportSize({ width: 834, height: 1112 });
  await page.goto("/");
  const scroller = page.locator("[data-grid-scroll]");
  await page.locator("[data-index]").first().waitFor();

  await scroller.evaluate((el) => {
    el.scrollTop = Math.min(900, el.scrollHeight - el.clientHeight);
  });
  // The grid records its anchor in the same throttled callback that saves the
  // offset, so wait for that write rather than racing it.
  await expect
    .poll(() =>
      page.evaluate(() => {
        const hit = Object.entries(sessionStorage).find(([k]) => k.startsWith("exviewer.scroll"));
        return hit ? Number(hit[1]) : -1;
      }),
    )
    .toBeGreaterThan(500);

  const topCard = await scroller.evaluate((el) => {
    const top = el.getBoundingClientRect().top;
    const cards = [...el.querySelectorAll('a[href^="/gallery/"]')];
    return cards.find((c) => c.getBoundingClientRect().top >= top - 2)?.getAttribute("href") ?? null;
  });
  expect(topCard).not.toBeNull();

  await page.setViewportSize({ width: 1112, height: 834 });

  // Where the card sits relative to the top of the feed, and how tall a row is
  // now, so "in the first row" is measured against the rotated layout.
  const placed = () =>
    scroller.evaluate((el, href) => {
      const card = el.querySelector(`a[href="${href}"]`);
      const row = el.querySelector("[data-index]");
      if (!card || !row) return null;
      return {
        offset: Math.round(card.getBoundingClientRect().top - el.getBoundingClientRect().top),
        rowH: Math.round(row.getBoundingClientRect().height),
      };
    }, topCard);

  // Re-anchoring runs off the resize, so wait for it before reading numbers.
  await expect.poll(async () => {
    const p = await placed();
    return !!p && p.offset >= -2 && p.offset < p.rowH;
  }).toBe(true);

  const { offset, rowH } = (await placed())!;
  expect(offset).toBeGreaterThanOrEqual(-2);
  expect(offset).toBeLessThan(rowH);
});
