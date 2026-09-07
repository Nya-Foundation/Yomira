// Run against `bun run dev`. Captures the browser demo with original, SFW artwork.
import { chromium, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';

const origin = 'http://localhost:1420';
const output = new URL('../docs/screenshots/', import.meta.url);
await mkdir(output, { recursive: true });
const browser = await chromium.launch();
try {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'en-US' });
  context.setDefaultTimeout(30_000);
  // A fresh browser profile and local-only requests keep captures independent of accounts.
  await context.route('**/*', route => {
    const url = new URL(route.request().url());
    return url.origin === origin || url.protocol === 'data:' ? route.continue() : route.abort();
  });
  await context.addInitScript(() => {
    if (sessionStorage.getItem('screenshots-seeded')) return;
    sessionStorage.setItem('screenshots-seeded', '1');
    sessionStorage.setItem('exviewer.liststate.home', JSON.stringify({
      params: { categories: ['Non-H'] }, searchInput: '', startPage: 0,
    }));
    localStorage.setItem('exviewer.settings', JSON.stringify({ theme: 'dark', language: 'en' }));
    localStorage.setItem('exviewer.downloads.seeded', '1');
  });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));

  async function capture(name, { images = true } = {}) {
    await page.evaluate(() => document.fonts.ready);
    // Wait for every visible image, including images still represented by a loading skeleton.
    await page.waitForFunction(requireImages => {
      const visible = element => {
        const rect = element.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0 && rect.top < innerHeight && rect.bottom > 0
          && rect.left < innerWidth && rect.right > 0 && getComputedStyle(element).visibility !== 'hidden';
      };
      if ([...document.querySelectorAll('[role="img"], .animate-pulse, .animate-spin')].some(visible)) return false;
      const images = [...document.images].filter(visible);
      return (!requireImages || images.length > 0) && images.every(image => image.complete && image.naturalWidth > 0);
    }, images);
    await page.screenshot({ path: new URL(name, output).pathname, animations: 'disabled' });
    console.log(`Captured ${name}`);
  }

  await page.goto(origin);
  await expect(page.locator('a[data-density]').first()).toBeVisible();
  await capture('gallery-list.png');
  const galleries = await page.evaluate(async () => {
    const { mock } = await import('/src/lib/mock.ts');
    const { DEFAULT_LIST_PARAMS } = await import('/src/lib/types.ts');
    return (await mock.getGalleryList('home', { ...DEFAULT_LIST_PARAMS, categories: ['Non-H'], page: 0 })).galleries;
  });
  const gallery = galleries[0];
  const detail = `/gallery/${gallery.gid}/${gallery.token}`;
  const reader = `/read/${gallery.gid}/${gallery.token}`;
  await page.goto(origin + detail);
  await expect(page.getByRole('heading', { name: gallery.title, exact: true })).toBeVisible();
  await capture('gallery-detail.png');
  await page.goto(origin + reader);
  await expect(page.getByRole('slider', { name: 'Page', exact: true })).toBeVisible();
  await capture('reader.png');

  await page.evaluate(galleries => {
    localStorage.setItem('exviewer.download-labels', JSON.stringify(['Field notes']));
    localStorage.setItem('exviewer.downloads', JSON.stringify(galleries.slice(0, 3).map((gallery, index) => ({
      gid: gallery.gid, token: gallery.token, state: index === 0 ? 'done' : 'paused',
      label: 'Field notes', finishedPages: index === 0 ? gallery.pages : 12 + index * 9,
      error: null, time: 1788739200000 - index * 3600000,
    }))));
  }, galleries);
  await page.goto(origin + '/downloads');
  await expect(page.getByRole('link', { name: /^Read offline:/ }).first()).toBeVisible();
  await capture('offline-downloads.png');
  await page.goto(origin + reader + '?source=download');
  await expect(page.getByRole('status').filter({ hasText: 'Offline' })).toBeVisible();
  await capture('offline-reader.png');

  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(origin);
  await expect(page.locator('a[data-density]').first()).toBeVisible();
  await capture('gallery-list-mobile.png');
  await page.getByRole('button', { name: 'More', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await capture('navigation-mobile.png', { images: false });
  await page.goto(origin + '/downloads');
  await expect(page.getByRole('link', { name: /^Read offline:/ }).first()).toBeVisible();
  await capture('downloads-mobile.png');
  await page.evaluate(() => localStorage.setItem('exviewer.settings', JSON.stringify({ theme: 'light', language: 'en' })));
  await page.goto(origin + '/settings');
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toBeVisible();
  await expect(page.locator('html')).toHaveClass(/light/);
  await expect(page.getByRole('combobox', { name: 'Theme', exact: true })).toHaveValue('light');
  await expect(page.getByRole('combobox', { name: 'Language', exact: true })).toBeVisible();
  await expect(page.getByRole('combobox', { name: 'Display scale', exact: true })).toBeVisible();
  await capture('settings-light-mobile.png', { images: false });
  if (errors.length) throw new Error(errors.join('\n'));
} finally {
  await browser.close();
}
