import { expect, test } from "@playwright/test";

test.use({ locale: "en-US" });

const firstTitle = (page: import("@playwright/test").Page) =>
  page.locator('a[href^="/gallery/"]').first().locator("img").getAttribute("alt");

test("the toplist range can be chosen and is restored on return", async ({ page }) => {
  await page.goto("/toplist");

  // Default range is the past month, and every range is offered.
  await expect(page.getByRole("tab", { name: "Past month" })).toHaveAttribute("aria-selected", "true");
  for (const label of ["Yesterday", "Past month", "Past year", "All time"]) {
    await expect(page.getByRole("tab", { name: label })).toBeVisible();
  }

  const monthly = await firstTitle(page);
  await page.getByRole("tab", { name: "Yesterday" }).click();
  await expect(page.getByRole("tab", { name: "Yesterday" })).toHaveAttribute("aria-selected", "true");
  // The range reaches the list request: a different ranking comes back.
  await expect.poll(() => firstTitle(page)).not.toBe(monthly);

  // The choice is part of the saved list state, so it survives a round trip.
  await expect
    .poll(() => page.evaluate(() => sessionStorage.getItem("exviewer.liststate.toplist")))
    .toContain('"timespan":"yesterday"');
  await page.locator('a[href^="/gallery/"]').first().click();
  await expect(page).toHaveURL(/\/gallery\//);
  await page.goBack();
  await expect(page).toHaveURL(/\/toplist$/);
  await expect(page.getByRole("tab", { name: "Yesterday" })).toHaveAttribute("aria-selected", "true");
});
