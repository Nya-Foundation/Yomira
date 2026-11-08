/**
 * Border, placeholder, hover, focus and disabled treatment shared by the text
 * fields. Size and padding stay with each component, since those are what
 * actually differ between a one-line input and a textarea.
 */
export const FIELD_BASE = [
  "w-full rounded-lg border border-control-border bg-surface text-base text-foreground",
  "placeholder:text-faint-foreground",
  "transition-colors duration-150 hover:border-border-strong",
  "focus:border-accent",
  "disabled:cursor-not-allowed disabled:opacity-50",
];
