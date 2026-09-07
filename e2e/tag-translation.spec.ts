import { expect, test } from "@playwright/test";

// Runs only in the chromium-zh project (locale zh-CN). The app translates tag
// pills only when the effective locale is zh-CN, which it derives from
// navigator.language when no language preference is persisted.

test.describe("tag translation (zh-CN)", () => {
  test("tag pills show zh-CN translations from the mock dictionary", async ({
    page,
  }) => {
    await page.goto("/gallery/1000001/486ef");

    const tagsSection = page.getByRole("region", { name: "标签" });
    await expect(tagsSection).toBeVisible({ timeout: 15_000 });

    // gid 1000001 has female:maid / female:glasses / misc:full_color among
    // its mock tags; each has a fixed entry in MOCK_TAG_TRANSLATIONS.
    // The pill flips from the raw tag to the translation once the batched
    // translateTags request resolves, so poll with a generous timeout.
    await expect(tagsSection.getByRole("link", { name: "女仆" })).toBeVisible({
      timeout: 10_000,
    });
    await expect(tagsSection.getByRole("link", { name: "眼镜" })).toBeVisible();
    await expect(tagsSection.getByRole("link", { name: "全彩" })).toBeVisible();
  });
});
