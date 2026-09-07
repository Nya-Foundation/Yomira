import { expect, test } from "@playwright/test";

test.describe("page jump", () => {
  test("jump dialog navigates the gallery grid to another page", async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") consoleErrors.push(msg.text());
    });
    page.on("pageerror", (err) => consoleErrors.push(String(err)));

    await page.goto("/");

    const cards = page.locator("main a[href^='/gallery/']");
    await expect(cards.first()).toBeVisible({ timeout: 15_000 });
    const firstTitleBefore = await cards.first().locator("p").first().innerText();

    await page.getByRole("button", { name: "Jump to page" }).click();
    const pageInput = page.locator("#jump-page");
    await expect(pageInput).toBeVisible();
    await pageInput.fill("2");
    await page.getByRole("button", { name: "Go", exact: true }).click();

    // grid reloads from the jumped-to page; first card should change and the
    // grid must be populated again
    await expect(cards.first()).toBeVisible({ timeout: 15_000 });
    await expect
      .poll(async () => cards.first().locator("p").first().innerText(), {
        timeout: 15_000,
      })
      .not.toBe(firstTitleBefore);
    await expect
      .poll(async () => cards.count(), { timeout: 15_000 })
      .toBeGreaterThanOrEqual(1);

    expect(consoleErrors).toEqual([]);
  });
});
