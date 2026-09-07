import { expect, test, type Page } from "@playwright/test";

const SEARCH_HISTORY_KEY = "exviewer.search-history";

async function openHomeWithSeededHistory(page: Page) {
  // Seed search history before any app script runs (mock history is an empty
  // localStorage list on a fresh browser otherwise).
  await page.addInitScript((key) => {
    localStorage.setItem(key, JSON.stringify(["midnight garden", "rating:>=4"]));
  }, SEARCH_HISTORY_KEY);
  await page.goto("/");
  const searchInput = page.getByRole("combobox", { name: "Search galleries" });
  await searchInput.click();
  return searchInput;
}

test.describe("search", () => {
  test("typing a tag query shows the autocomplete dropdown with suggestions", async ({
    page,
  }) => {
    const searchInput = await openHomeWithSeededHistory(page);

    await searchInput.fill("female:s");

    const listbox = page.getByRole("listbox", { name: "Tag suggestions" });
    await expect(listbox).toBeVisible({ timeout: 10_000 });
    // mock suggestion dictionary always includes this namespace-narrowed hit
    await expect(listbox.getByRole("option", { name: /sole_female/ })).toBeVisible();
  });

  test("focusing the empty search input shows the search-history dropdown", async ({
    page,
  }) => {
    await openHomeWithSeededHistory(page);

    await expect(page.getByText("Recent searches")).toBeVisible({ timeout: 10_000 });
    await expect(
      page.getByRole("button", { name: "Search for midnight garden" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Search for rating:>=4" }),
    ).toBeVisible();
  });
});
