import type { ComponentProps, ReactNode } from "react";
import { cn } from "../../lib/utils";

/**
 * Deliberately not floored to 44px like the other controls: the coarse-pointer
 * rule in index.css excludes checkboxes, because a 44px box reads as a mistake
 * beside its own label. `CheckboxField` gives the row the target instead.
 */
export function Checkbox({ className, ...props }: ComponentProps<"input">) {
  return <input type="checkbox" className={cn("h-4 w-4 accent-accent", className)} {...props} />;
}

/** A checkbox and its label as one row, so the text toggles it too. */
export function CheckboxField({
  label,
  className,
  ...props
}: ComponentProps<"input"> & { label: ReactNode }) {
  return (
    <label
      className={cn(
        "flex cursor-pointer items-center gap-2 text-small text-muted-foreground select-none",
        className,
      )}
    >
      <Checkbox {...props} />
      {label}
    </label>
  );
}
