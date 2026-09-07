import { expect, test } from "@playwright/test";
import { installFixture } from "../scripts/profiling/fixture.mjs";

test("native offline reading refreshes on download events and does no idle polling", async ({ page }) => {
  await page.addInitScript(installFixture);
  const requests: string[] = [];
  page.on("request", request => requests.push(request.url()));
  await page.goto("/read/1/fixture?source=download");
  await expect(page.getByRole("status").filter({ hasText: "240 of 240 pages saved" })).toBeVisible();
  const calls = () => page.evaluate(() => (window as unknown as { __profileCalls: Record<string, number> }).__profileCalls.get_downloaded_gallery);
  const initial = await calls();
  await page.waitForTimeout(5200); // Deliberately exceeds the former five-second polling interval.
  expect(await calls()).toBe(initial);
  await page.evaluate(() => (window as unknown as { __profileEmit: (event: string) => void }).__profileEmit("download://changed"));
  await expect.poll(calls).toBe(initial + 1);
  expect(requests.some(url => /\/src\/lib\/mock(?:\.ts|\/)/.test(url))).toBe(false);

  await page.getByRole("button", { name: "Go back", exact: true }).click();
  await expect(page).toHaveURL(/\/downloads$/);
  await expect(page.getByRole("button", { name: "New label", exact: true })).toBeVisible();
  const afterLeaving = await calls();
  await page.evaluate(() => (window as unknown as { __profileEmit: (event: string) => void }).__profileEmit("download://changed"));
  expect(await calls()).toBe(afterLeaving);
});
