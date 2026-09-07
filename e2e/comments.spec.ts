import { expect, test } from "@playwright/test";

test.describe("comment rendering", () => {
  test("BBCode url becomes a real anchor and <br> line breaks are preserved", async ({
    page,
  }) => {
    await page.goto("/gallery/1000001/486ef");

    const commentsSection = page.getByRole("region", { name: "Comments" });
    await expect(commentsSection).toBeVisible({ timeout: 15_000 });

    // [url=...] BBCode in the mock's second comment must render as a link,
    // never as literal "[url" text
    const link = commentsSection.getByRole("link", {
      name: "shimura0404's page",
    });
    await expect(link).toBeVisible({ timeout: 10_000 });
    await expect(link).toHaveAttribute("href", "https://example.org/shimura0404");
    await expect(commentsSection).not.toContainText("[url");

    // the first mock comment contains a <br> that must survive as a break
    const firstComment = commentsSection
      .locator("li")
      .filter({ hasText: "Fantastic art" })
      .first();
    await expect(firstComment).toBeVisible();
    await expect(firstComment.locator("br")).toHaveCount(1);
  });
});
