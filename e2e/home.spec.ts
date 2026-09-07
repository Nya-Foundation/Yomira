import { expect, test } from "@playwright/test";

test.describe("home", () => {
  test("renders the app shell and eventually shows gallery cards", async ({ page }) => {
    await page.goto("/");

    // app shell: sidebar rail with nav links, header, search input
    const nav = page.getByRole("navigation", { name: "Primary" });
    await expect(nav).toBeVisible();
    await expect(page.getByRole("link", { name: "Home" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Popular" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Favorites" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Home" })).toBeVisible();
    await expect(page.getByRole("combobox", { name: "Search galleries" })).toBeVisible();

    // mock list has a ~450ms artificial delay; wait for the grid to populate
    const cards = page.locator("main a[href^='/gallery/']");
    await expect(cards.first()).toBeVisible({ timeout: 15_000 });
    await expect
      .poll(async () => cards.count(), { timeout: 15_000 })
      .toBeGreaterThanOrEqual(1);
    // every card links to a detail route and carries a non-empty title
    const firstTitle = cards.first().locator("p").first();
    await expect(firstTitle).not.toBeEmpty();
  });
});
