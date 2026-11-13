import { expect, test } from "@playwright/test";

test.use({ locale: "en-US" });

test("a successful sign-in refreshes the account state in place", async ({ page }) => {
  await page.goto("/signin");
  const account = page.getByRole("link", { name: "Account / sign in" });
  await expect(account).toBeVisible();
  // Signed out: the account button shows the outline icon, not the avatar.
  await expect(account.getByText("U", { exact: true })).toHaveCount(0);

  await page
    .getByRole("textbox")
    .fill("ipb_member_id=1234567; ipb_pass_hash=abcdef0123456789");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page).toHaveURL(/\/$/);

  // The sidebar stays mounted across the navigation, so this only passes when
  // the session query was invalidated instead of left stale until the window
  // is focused again.
  await expect(account.getByText("U", { exact: true })).toBeVisible();
});
