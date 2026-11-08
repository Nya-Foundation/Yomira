import { clsx, type ClassValue } from "clsx";
import { currentLocale, t } from "./i18n";
import { extendTailwindMerge } from "tailwind-merge";

// Custom typography tokens are font sizes, not text colors. Without this,
// merging `text-small` can silently remove a button's foreground color.
const twMerge = extendTailwindMerge({
  extend: {
    classGroups: {
      "font-size": [{ text: ["caption", "small", "emphasis", "title", "heading", "display"] }],
    },
  },
});

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = "B";
  for (const u of units) {
    if (value < 1024) break;
    value /= 1024;
    unit = u;
  }
  return `${value.toFixed(1)} ${unit}`;
}

/**
 * Posted/commented times as read by a person. Callers render this inside
 * components that subscribe through `useT()`, so switching language
 * re-renders them in the new one.
 */
export function formatRelativeTime(epochSeconds: number): string {
  const diff = Date.now() / 1000 - epochSeconds;
  if (diff < 60) return t("time.justNow");
  if (diff < 3600) return t("time.minutes", { n: Math.floor(diff / 60) });
  if (diff < 86400) return t("time.hours", { n: Math.floor(diff / 3600) });
  if (diff < 86400 * 30) return t("time.days", { n: Math.floor(diff / 86400) });
  // Older than a month: an absolute date, formatted for the chosen language
  // rather than whatever the webview's own locale happens to be.
  return new Date(epochSeconds * 1000).toLocaleDateString(currentLocale());
}

export function formatCount(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return `${n}`;
}

/** True when the event originates from a text-editing control. */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT" ||
    target.isContentEditable
  );
}
