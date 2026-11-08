import { beforeEach, expect, test } from "bun:test";
import { applyRememberedPaint, useSettings } from "../src/lib/settings";

const classes = new Set<string>();
const style: Record<string, string> = {};
const stored = new Map<string, string>();

function stubDom(localStorage: unknown = {
  getItem: (key: string) => stored.get(key) ?? null,
  setItem: (key: string, value: string) => void stored.set(key, value),
}) {
  Object.assign(globalThis, {
    localStorage,
    document: {
      documentElement: {
        classList: {
          toggle: (name: string, on: boolean) => void (on ? classes.add(name) : classes.delete(name)),
        },
        style,
      },
    },
  });
}

beforeEach(() => {
  classes.clear();
  stored.clear();
  for (const key of Object.keys(style)) delete style[key];
  stubDom();
});

test("what the last session chose is applied before mount", () => {
  // The persisted copy answers over IPC, and an Android cold start drops that
  // first response, so the first paint cannot depend on it arriving at all.
  useSettings.getState().update({ theme: "light", density: "compact", displayScale: 1.5 });
  classes.clear();
  delete style.fontSize;

  applyRememberedPaint();
  expect(classes.has("light")).toBe(true);
  expect(classes.has("dense")).toBe(true);
  expect(style.fontSize).toBe(`${14 * 1.5}px`);
});

test("with nothing remembered the dark-first defaults stand", () => {
  applyRememberedPaint();
  expect(classes.has("light")).toBe(false);
  expect(classes.has("dense")).toBe(false);
  expect(style.fontSize).toBeUndefined();
});

test("a theme remembered by an older build still applies", () => {
  stored.set("exviewer.theme", "light");
  applyRememberedPaint();
  expect(classes.has("light")).toBe(true);
});

test("unreadable storage must not break the first paint", () => {
  stubDom({
    getItem() {
      throw new Error("storage blocked");
    },
  });
  expect(() => applyRememberedPaint()).not.toThrow();
});
