import { expect, test } from "@playwright/test";

const COMMENTERS =
  /archive_rat|page_turner|quiet_reader|melon_bread|exv_fan|thumbnail enjoyer/;

test.describe("gallery detail", () => {
  test("detail page renders title, tag pills and comments with authors", async ({
    page,
  }) => {
    await page.goto("/");

    // open the first gallery card's detail page through the real link
    const firstCard = page.locator("main a[href^='/gallery/']").first();
    await expect(firstCard).toBeVisible({ timeout: 15_000 });
    const cardTitle = await firstCard.locator("p").first().innerText();
    await firstCard.click();

    await expect(page).toHaveURL(/\/gallery\/\d+\//);
    const title = page.getByRole("heading", { level: 2 });
    await expect(title).toHaveText(cardTitle, { timeout: 15_000 });

    // tag pills
    const tagsSection = page.getByRole("region", { name: "Tags" });
    await expect(tagsSection).toBeVisible();
    const tagPills = tagsSection.locator("a[href^='/?search=']");
    await expect
      .poll(async () => tagPills.count(), { timeout: 10_000 })
      .toBeGreaterThanOrEqual(1);

    // comments section with authors
    const commentsSection = page.getByRole("region", { name: "Comments" });
    await expect(commentsSection).toBeVisible();
    const firstComment = commentsSection.locator("li").first();
    await expect(firstComment).toBeVisible({ timeout: 10_000 });
    await expect(firstComment).toContainText(COMMENTERS);
    // mock detail page 1 always embeds this comment body
    await expect(commentsSection).toContainText(
      "Fantastic art, the composition on every page is immaculate.",
    );
  });
});
