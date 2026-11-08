import { expect, test } from "@playwright/test";

// Each toggle used to write straight into the list query, so composing a
// filter set queued one rate-limited listing request per tap — for states the
// user was still assembling, ahead of the one they wanted. Edits are now held
// in the sheet and committed once on exit, which also gives "Apply" a job.
test("category filters apply once, on leaving the sheet", async ({ page }) => {
  await page.goto("/");
  const categories = page.getByRole("button", { name: "Category filters" });
  await categories.click();

  const sheet = page.locator("dialog[open]");
  await expect(sheet).toBeVisible();
  const chips = sheet.locator("button[aria-pressed]");
  await chips.nth(0).click();
  await chips.nth(1).click();

  // Both chips read as selected inside the sheet...
  await expect(chips.nth(0)).toHaveAttribute("aria-pressed", "true");
  await expect(chips.nth(1)).toHaveAttribute("aria-pressed", "true");
  // ...while the toolbar still shows no active filter count, because nothing
  // has been committed and so no new listing has been requested.
  await expect(categories).not.toContainText("2");

  await page.keyboard.press("Escape");
  await expect(sheet).toBeHidden();
  await expect(categories).toContainText("2");
});
