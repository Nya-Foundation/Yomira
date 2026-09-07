import { expect, test } from "@playwright/test";
import { installFixture } from "../scripts/profiling/fixture.mjs";

test("autocomplete adds bare second and third tags and edits a quoted middle term", async ({ page }) => {
  await page.goto("/");
  const input = page.getByRole("combobox", { name: "Search galleries", exact: true });
  await input.fill("language:chinese tou");
  const suggestions = page.getByRole("listbox", { name: "Tag suggestions" });
  await suggestions.getByRole("option", { name: /touhou_project/ }).click();
  await expect(input).toHaveValue("language:chinese parody:touhou_project ");
  await input.pressSequentially("chin");
  await expect(suggestions.getByRole("option", { name: /chinese/ })).toBeVisible();
  await input.press("Enter");
  await expect(input).toHaveValue("language:chinese parody:touhou_project language:chinese ");
  await input.fill('language:chinese -parody:"tou hou" language:chinese');
  await input.press("Home");
  for (let i = 0; i < 'language:chinese -parody:"tou'.length; i++) await input.press("ArrowRight");
  await suggestions.getByRole("option", { name: /touhou_project/ }).click();
  await expect(input).toHaveValue("language:chinese -parody:touhou_project language:chinese");
});

test("native autocomplete completes namespaces without spaces and recovers from a failed request", async ({ page }) => {
  await page.addInitScript(installFixture);
  await page.addInitScript(() => {
    const w = window as any, original = w.__TAURI_INTERNALS__.invoke;
    w.__TAURI_INTERNALS__.invoke = async (command: string, args: any) => {
      if (command === "tag_db_status") return { available: true, version: "fixture", entryCount: 2 };
      if (command === "suggest_tags") {
        if (args.query === "fail") throw new Error("offline");
        return [{ namespace: "language", name: args.query === "lan" ? "" : "english", translated: "" }];
      }
      return original(command, args);
    };
  });
  await page.goto("/");
  const input = page.getByRole("combobox", { name: "Search galleries", exact: true });
  await input.fill("fail");
  await expect(page.getByText("Tag database unavailable (offline?)", { exact: true })).toBeVisible();
  await input.fill("language:chinese lan");
  await page.getByRole("option").click();
  await expect(input).toHaveValue("language:chinese language:");
  await page.getByRole("option", { name: /english/ }).click();
  await expect(input).toHaveValue("language:chinese language:english ");
});

test("half-star filters reach native IPC and continue through an empty filtered page", async ({ page }) => {
  await page.addInitScript(installFixture);
  await page.addInitScript(() => {
    const w = window as any, original = w.__TAURI_INTERNALS__.invoke;
    w.__ratings = [];
    w.__TAURI_INTERNALS__.invoke = async (command: string, args: any) => {
      if (command !== "get_gallery_list") return original(command, args);
      w.__ratings.push(args.params.minRating);
      const result = await original(command, args);
      return { ...result, galleries: args.params.minRating && args.params.page === 0 ? [] : result.galleries.map((g: any) => ({ ...g, rating: 5 })), nextPage: args.params.page === 0 ? 1 : null };
    };
  });
  await page.goto("/");
  for (const rating of ["3.5", "4.5", "5"]) {
    await page.getByRole("button", { name: "Advanced search", exact: true }).click();
    await page.getByLabel("Minimum rating", { exact: true }).selectOption(rating);
    await page.getByRole("dialog").getByRole("button", { name: "Close", exact: true }).click();
    await expect.poll(() => page.evaluate(() => (window as any).__ratings)).toContain(Number(rating));
    await expect(page.locator('main a[href="/gallery/26/fixture"]')).toBeVisible();
  }
});

for (const width of [390, 1366]) {
  test(`history controls preserve search state, reader return and forward branches at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    const back = page.getByRole("button", { name: "Go back", exact: true });
    const forward = page.getByRole("button", { name: "Go forward", exact: true });
    await expect(back).toBeDisabled();
    await expect(forward).toBeDisabled();
    for (const control of [back, forward]) {
      const bounds = await control.boundingBox();
      expect(bounds!.width).toBeGreaterThanOrEqual(44);
      expect(bounds!.height).toBeGreaterThanOrEqual(44);
    }
    const input = page.getByRole("combobox", { name: "Search galleries", exact: true });
    await input.fill("garden");
    await input.press("Escape");
    await expect.poll(() => page.evaluate(() => JSON.parse(sessionStorage.getItem("exviewer.liststate.home")!).params.search)).toBe("garden");
    const gallery = page.locator('main a[href^="/gallery/"]').first();
    await gallery.click();
    const detailUrl = page.url();
    await page.getByRole("button", { name: "Read now", exact: true }).click();
    await expect(page.getByRole("slider", { name: "Page", exact: true })).toBeVisible();
    await back.click();
    await expect(page).toHaveURL(detailUrl);
    await expect(forward).toBeEnabled();
    await page.keyboard.press("Alt+ArrowLeft");
    await expect(input).toHaveValue("garden");
    await page.reload();
    await expect(forward).toBeEnabled();
    await page.keyboard.press("Alt+ArrowRight");
    await expect(page).toHaveURL(detailUrl);
    await back.click();
    await page.getByRole("link", { name: "Downloads", exact: true }).click();
    await expect(forward).toBeDisabled();
    const handled = await page.evaluate(() => !window.dispatchEvent(new Event("yomira:back", { cancelable: true })));
    expect(handled).toBe(true);
    await expect(input).toHaveValue("garden");
    await page.getByRole("button", { name: "Advanced search", exact: true }).click();
    await page.evaluate(() => window.dispatchEvent(new Event("yomira:back", { cancelable: true })));
    await expect(page.getByRole("dialog")).not.toBeVisible();
    await expect(input).toBeVisible();
  });
}

test("desktop shortcuts scale the interface, persist it and reset without native browser zoom", async ({ page }) => {
  await page.goto("/");
  const input = page.getByRole("combobox", { name: "Search galleries", exact: true });
  await input.click();
  await page.keyboard.press("Control+=");
  const scale = () => page.evaluate(() => JSON.parse(localStorage.getItem("exviewer.settings") ?? "{}").displayScale);
  await expect.poll(scale).toBe(1.1);
  await page.keyboard.press("Control+-");
  await expect.poll(scale).toBe(1);
  await page.keyboard.down("Control");
  await page.mouse.wheel(0, -120);
  await page.keyboard.up("Control");
  await expect.poll(scale).toBe(1.1);
  await page.reload();
  await expect.poll(() => page.evaluate(() => parseFloat(getComputedStyle(document.documentElement).fontSize))).toBeCloseTo(15.4);
  await page.keyboard.press("Control+0");
  await expect.poll(scale).toBe(1);
  await expect.poll(() => page.evaluate(() => window.visualViewport?.scale)).toBe(1);
});

for (const mode of ["paged", "continuous"]) {
  test(`reader zoom shortcuts affect the image and preserve the current page in ${mode} mode`, async ({ page }) => {
    await page.addInitScript(installFixture);
    await page.addInitScript(mode => {
      localStorage.setItem("profile-settings", JSON.stringify({ readerMode: mode }));
      localStorage.setItem("exviewer.reader.pos.1", "10");
    }, mode);
    await page.goto("/read/1/fixture?source=download");
    const slider = page.getByRole("slider", { name: "Page", exact: true });
    await expect(slider).toHaveValue("10");
    const imageScale = () => mode === "paged"
      ? page.getByRole("img", { name: "Page 11", exact: true }).evaluate(el => el.style.transform)
      : page.locator("[data-reader-zoom]").getAttribute("data-reader-zoom");
    await page.keyboard.press("Control+=");
    await expect.poll(imageScale).toContain("1.1");
    await page.mouse.move(500, 450);
    await page.keyboard.down("Control");
    await page.mouse.wheel(0, -120);
    await page.keyboard.up("Control");
    await expect.poll(imageScale).toContain("1.2");
    await expect(slider).toHaveValue("10");
    await page.keyboard.press("Control+0");
    await expect.poll(imageScale).toBe(mode === "paged" ? "translate(0px, 0px) scale(1)" : "1");
    expect(await page.evaluate(() => getComputedStyle(document.documentElement).fontSize)).toBe("14px");
  });
}
