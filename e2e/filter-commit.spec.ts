import { expect, test } from "@playwright/test";

// Each toggle used to write straight into the list query, so composing a
// filter set queued one rate-limited listing request per tap — for states the
// user was still assembling, ahead of the one they wanted. Edits are now held
// in the sheet, and Apply is the only thing that commits them.
test("category filters commit on Apply, not on every tap", async ({ page }) => {
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

  await sheet.getByRole("button", { name: "Apply" }).click();
  await expect(sheet).toBeHidden();
  await expect(categories).toContainText("2");
});

// Dismissing a sheet that has an explicit Apply button has to mean "never
// mind" — it used to commit, so Escape silently ran the search the user was
// backing out of.
test("dismissing the sheet discards the edits", async ({ page }) => {
  await page.goto("/");
  const categories = page.getByRole("button", { name: "Category filters" });
  await categories.click();

  const sheet = page.locator("dialog[open]");
  const chips = sheet.locator("button[aria-pressed]");
  await chips.nth(0).click();
  await expect(chips.nth(0)).toHaveAttribute("aria-pressed", "true");

  await page.keyboard.press("Escape");
  await expect(sheet).toBeHidden();
  await expect(categories).not.toContainText("1");

  // Reopening shows the committed state, not the abandoned draft.
  await categories.click();
  await expect(sheet.locator("button[aria-pressed]").nth(0)).toHaveAttribute(
    "aria-pressed",
    "false",
  );
});
