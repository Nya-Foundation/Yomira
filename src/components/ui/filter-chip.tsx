import { cn } from "../../lib/utils";

/** Small rounded filter pill (favorites slots, download labels, ...). */
export function FilterChip({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "h-7 rounded-full border px-2.5 text-small transition-colors duration-150",
        active
          ? "border-accent/40 bg-accent/15 text-accent"
          : "border-border bg-surface text-muted-foreground hover:text-foreground",
      )}
    >
      {label}
    </button>
  );
}
