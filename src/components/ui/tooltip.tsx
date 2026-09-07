import type { ReactNode } from "react";
import { cn } from "../../lib/utils";

/** Pure-CSS hover tooltip. Wrap the trigger; content shows on hover/focus-within. */
export function Tooltip({
  content,
  children,
  side = "right",
  className,
}: {
  content: ReactNode;
  children: ReactNode;
  side?: "right" | "top" | "bottom" | "left";
  className?: string;
}) {
  const sideClass = {
    right: "left-full top-1/2 -translate-y-1/2 ml-2",
    left: "right-full top-1/2 -translate-y-1/2 mr-2",
    top: "bottom-full left-1/2 -translate-x-1/2 mb-2",
    bottom: "top-full left-1/2 -translate-x-1/2 mt-2",
  }[side];
  return (
    <span className={cn("group/tt relative inline-flex", className)}>
      {children}
      <span
        role="tooltip"
        className={cn(
          "pointer-events-none absolute z-50 whitespace-nowrap rounded-md border border-border",
          "bg-overlay px-2 py-1 text-caption text-foreground opacity-0",
          "transition-opacity duration-150 group-hover/tt:opacity-100 group-focus-within/tt:opacity-100",
          sideClass,
        )}
      >
        {content}
      </span>
    </span>
  );
}
