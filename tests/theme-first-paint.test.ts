import { beforeEach, expect, test } from "bun:test";
import { applyRememberedTheme, useSettings } from "../src/lib/settings";

const classes = new Set<string>();
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
        style: {},
      },
    },
  });
}

beforeEach(() => {
  classes.clear();
  stored.clear();
  stubDom();
});

test("the theme chosen last session is applied before mount", () => {
  // What `update` persists is what the next cold start can read synchronously.
  useSettings.getState().update({ theme: "light" });
  classes.clear();

  applyRememberedTheme();
  expect(classes.has("light")).toBe(true);
});

test("with nothing remembered the dark-first palette stands", () => {
  applyRememberedTheme();
  expect(classes.has("light")).toBe(false);
});

test("unreadable storage must not break the first paint", () => {
  stubDom({
    getItem() {
      throw new Error("storage blocked");
    },
  });
  expect(() => applyRememberedTheme()).not.toThrow();
});
