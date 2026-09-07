import { expect, test, type Page } from "@playwright/test";
import { installFixture } from "../scripts/profiling/fixture.mjs";

test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

async function swipe(page: Page, from: [number, number], to: [number, number]) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: from[0], y: from[1], id: 0 }] });
  for (let step = 1; step <= 6; step++) {
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ x: from[0] + (to[0] - from[0]) * step / 6, y: from[1] + (to[1] - from[1]) * step / 6, id: 0 }] });
  }
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await cdp.detach();
}

for (const direction of ["ltr", "rtl"]) {
  test(`touch swipes turn pages exactly once in ${direction} mode`, async ({ page }) => {
    await page.addInitScript(installFixture);
    await page.addInitScript((direction) => {
      localStorage.setItem("exviewer.reader.pos.1", "5");
      localStorage.setItem("profile-settings", JSON.stringify({ readerDirection: direction }));
    }, direction);
    await page.goto("/read/1/fixture?source=download");
    const slider = page.getByRole("slider", { name: "Page", exact: true });
    await expect(slider).toHaveValue("5");
    await swipe(page, [370, 450], [40, 450]); // Starts in an edge-tap area.
    await expect(slider).toHaveValue(direction === "ltr" ? "6" : "4");
    await swipe(page, [40, 450], [340, 450]);
    await expect(slider).toHaveValue("5");
    await swipe(page, [190, 500], [195, 280]);
    await expect(slider).toHaveValue("5");
    const cdp = await page.context().newCDPSession(page);
    await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: 330, y: 450, id: 0 }] });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ x: 180, y: 450, id: 0 }] });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchCancel", touchPoints: [] });
    await expect(slider).toHaveValue("5");
    await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: 150, y: 400, id: 0 }, { x: 240, y: 400, id: 1 }] });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ x: 100, y: 400, id: 0 }, { x: 290, y: 400, id: 1 }] });
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await cdp.detach();
    await expect(slider).toHaveValue("5");
    await expect(page.getByRole("button", { name: "reset", exact: true })).toBeVisible();
    await swipe(page, [300, 450], [80, 450]); // A zoomed image pans, never turns.
    await expect(slider).toHaveValue("5");
  });
}

test("native continuation links advance the listing and repeated cursors stop", async ({ page }) => {
  await page.addInitScript(installFixture);
  await page.addInitScript(() => {
    const w = window as any;
    const invoke = w.__TAURI_INTERNALS__.invoke;
    w.__listingRequests = [];
    w.__TAURI_INTERNALS__.invoke = async (command: string, args: any) => {
      if (command !== "get_gallery_list") return invoke(command, args);
      w.__listingRequests.push(args.params);
      const cursor = args.params.cursor;
      const index = cursor?.includes("next=800") ? 2 : cursor ? 1 : 0;
      const result = await invoke(command, { ...args, params: { ...args.params, page: index } });
      return { ...result, pages: 0, nextPage: `https://e-hentai.org/?next=${index === 0 ? 900 : 800}&f_search=example` };
    };
  });
  await page.goto("/");
  const scroll = page.locator("main [data-load-more]").locator("..");
  for (let i = 0; i < 3; i++) {
    await scroll.evaluate(el => { el.scrollTop = el.scrollHeight; });
    await page.waitForTimeout(200);
  }
  await expect(page.getByText("End of results", { exact: true })).toBeVisible();
  const requests = await page.evaluate(() => (window as any).__listingRequests);
  expect(requests).toHaveLength(3);
  expect(requests[1]).toMatchObject({ page: 0, cursor: "https://e-hentai.org/?next=900&f_search=example" });
  expect(requests[2].cursor).toContain("next=800");
  await page.getByRole("button", { name: "Detailed list", exact: true }).click();
  await expect(page.locator('[data-gallery-view="detail"]').first()).toBeVisible();
  await page.reload();
  await expect(page.getByRole("button", { name: "Detailed list", exact: true })).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator('[data-gallery-view="detail"]').first()).toBeVisible();
});

test("pagination retries a failed continuation without discarding loaded galleries", async ({ page }) => {
  await page.addInitScript(installFixture);
  await page.addInitScript(() => {
    const w = window as any;
    const invoke = w.__TAURI_INTERNALS__.invoke;
    w.__failNext = true;
    w.__TAURI_INTERNALS__.invoke = async (command: string, args: any) => {
      if (command === "get_gallery_list") {
        if (args.params.page === 1 && w.__failNext) throw { kind: "Network", message: "connection timed out" };
        const result = await invoke(command, args);
        return { ...result, nextPage: args.params.page === 0 ? 1 : null };
      }
      return invoke(command, args);
    };
  });
  await page.goto("/");
  const scroll = page.locator("main [data-load-more]").locator("..");
  await expect(page.locator('main a[href^="/gallery/"]').first()).toBeVisible();
  await scroll.evaluate(el => { el.scrollTop = el.scrollHeight; });
  const retry = page.getByRole("button", { name: "Retry", exact: true });
  await expect(retry).toBeVisible();
  await page.evaluate(() => { (window as any).__failNext = false; });
  await retry.click();
  await scroll.evaluate(el => { el.scrollTop = el.scrollHeight; });
  await expect(page.getByText("End of results", { exact: true })).toBeVisible();
  await scroll.evaluate(el => { el.scrollTop = 0; });
  await expect(page.locator('a[href="/gallery/1/fixture"]')).toBeVisible();
});

test("network settings import hosts, persist image quality and reject invalid rows", async ({ page }) => {
  await page.goto("/settings");
  await page.getByRole("tab", { name: "Network", exact: true }).click();
  await page.getByRole("button", { name: "Load built-in hosts", exact: true }).click();
  const editor = page.getByLabel("Hosts overrides", { exact: true });
  await expect(editor).toHaveValue(/e-hentai.org/);
  await page.getByLabel("Import hosts.txt", { exact: true }).setInputFiles({ name: "hosts.txt", mimeType: "text/plain", buffer: Buffer.from("1.2.3.4 gallery.example\n::1 localhost\n") });
  await page.getByLabel("Image resolution", { exact: true }).selectOption("1600");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem("exviewer.network") ?? "{}").imageResolution)).toBe("1600");
  await page.reload();
  await page.getByRole("tab", { name: "Network", exact: true }).click();
  await expect(page.getByLabel("Image resolution", { exact: true })).toHaveValue("1600");
  await expect(editor).toHaveValue(/1\.2\.3\.4 gallery\.example/);
  await editor.fill("999.1.1.1 bad.example");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect(page.getByText(/Line 1: expected/)).toBeVisible();
});

for (const preload of [0, 10]) {
  test(`reader honors a preload count of ${preload}`, async ({ page }) => {
    await page.addInitScript(installFixture);
    await page.addInitScript(preload => localStorage.setItem("profile-settings", JSON.stringify({ readerPreloadCount: preload })), preload);
    await page.goto("/read/1/fixture?source=download");
    await expect(page.getByRole("img", { name: "Page 1", exact: true })).toBeVisible();
    await expect.poll(() => page.evaluate(() => (window as any).__profileCalls.get_downloaded_page)).toBe(preload + 1);
  });
}
