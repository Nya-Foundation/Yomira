import type { Category } from "./types";
import { useSettings } from "./settings";

/** Eh category badge colors — lib constant, not design tokens. */
export const CATEGORY_COLORS: Record<Category, string> = {
  Doujinshi: "#fc4e4e",
  Manga: "#e78c3d",
  "Artist CG": "#c7a93c",
  "Game CG": "#8eba43",
  Western: "#4aac67",
  "Non-H": "#42ac88",
  "Image Set": "#4a9eac",
  Cosplay: "#6991d1",
  "Asian Porn": "#988ac3",
  Misc: "#af82bb",
};

/**
 * The same value is the badge's text, its 17% fill and its 55% border, so each
 * theme needs its own set: the dark values sit at 2.0-3.3:1 as text on a pale
 * tint. Solved for at least 4.5:1 against its own fill over *both* pale
 * surfaces — badges sit on the canvas as well as on cards, and solving only
 * against the card left eight of these at 4.34-4.47 on the canvas.
 */
export const CATEGORY_COLORS_LIGHT: Record<Category, string> = {
  Doujinshi: "#c00000",
  Manga: "#984f0f",
  "Artist CG": "#74621f",
  "Game CG": "#516b24",
  Western: "#2d6f41",
  "Non-H": "#286f57",
  "Image Set": "#306b74",
  Cosplay: "#3462ac",
  "Asian Porn": "#6b56a8",
  Misc: "#844f92",
};

export const FAVORITE_SLOTS = 10;

/** Restrained per-namespace tag colors (hex, muted to sit on the dark UI). */
export const NAMESPACE_COLORS: Record<string, string> = {
  female: "#c77ba2",
  male: "#6d9bc4",
  artist: "#a68cc9",
  character: "#c2a05c",
  group: "#72a88b",
  parody: "#c78f5c",
  language: "#8b95c9",
  mixed: "#7fa8a0",
  misc: "#8c919b",
};

export const NAMESPACE_COLORS_LIGHT: Record<string, string> = {
  female: "#a44274",
  male: "#386893",
  artist: "#784faf",
  character: "#7d622c",
  group: "#426d56",
  parody: "#8b5a2d",
  language: "#5161b1",
  mixed: "#496e67",
  misc: "#61666f",
};

/** Badge colours for the active theme. */
export function useBadgeColors() {
  const light = useSettings((s) => s.settings.theme) === "light";
  const cats = light ? CATEGORY_COLORS_LIGHT : CATEGORY_COLORS;
  const names = light ? NAMESPACE_COLORS_LIGHT : NAMESPACE_COLORS;
  return {
    category: (c: Category) => cats[c],
    namespace: (n: string) => names[n] ?? names.misc,
  };
}
