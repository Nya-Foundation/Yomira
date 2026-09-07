import type { Category } from "./types";

/** Eh category badge colors — lib constant, not design tokens. */
export const CATEGORY_COLORS: Record<Category, string> = {
  Doujinshi: "#fc4e4e",
  Manga: "#e78c3d",
  "Artist CG": "#c7a93c",
  "Game CG": "#8eba43",
  Western: "#4aac67",
  "Non-H": "#42ac88",
  "Image Set": "#4a9eac",
  Cosplay: "#5480c7",
  "Asian Porn": "#7663ac",
  Misc: "#925aa0",
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
  misc: "#8a8f98",
};
