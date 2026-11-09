import { expect, test } from "bun:test";
import { imageSrc, remoteImageSrc } from "../src/lib/image-source";

const BASE = "http://127.0.0.1:41234";

test("the whole target becomes one opaque value the server can decode", () => {
  const src = remoteImageSrc(BASE, "https://ehgt.org/t/00/ab/cover.webp");
  expect(src).toBe(`${BASE}/remote?u=https%3A%2F%2Fehgt.org%2Ft%2F00%2Fab%2Fcover.webp`);
  // Everything after `?u=` is decoded as one value, so a target carrying its
  // own query must not be able to introduce further parameters.
  const tricky = remoteImageSrc(BASE, "https://ehgt.org/t/x.jpg?a=1&b=2#frag");
  expect(tricky.slice(`${BASE}/remote?u=`.length)).not.toInclude("&");
  expect(tricky.slice(`${BASE}/remote?u=`.length)).not.toInclude("=");
  expect(decodeURIComponent(tricky.slice(`${BASE}/remote?u=`.length))).toBe(
    "https://ehgt.org/t/x.jpg?a=1&b=2#frag",
  );
});

test("outside the app, fixture data URLs and served URLs pass through", () => {
  expect(imageSrc("data:image/svg+xml,<svg/>")).toBe("data:image/svg+xml,<svg/>");
  expect(imageSrc(`${BASE}/local-0123456789abcdef.avif`)).toBe(
    `${BASE}/local-0123456789abcdef.avif`,
  );
  expect(imageSrc("")).toBeNull();
});
