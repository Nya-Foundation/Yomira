import type { ReactNode } from "react";
import { cn } from "../../lib/utils";

/** The bar beneath the header on list pages: search, filters, chip rows. */
export function PageToolbar({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex shrink-0 flex-col gap-2 border-b border-border px-4 pb-3 pt-3",
        className,
      )}
    >
      {children}
    </div>
  );
}

/**
 * A row that scrolls sideways instead of wrapping, for chips and small
 * controls. The bottom padding keeps the scrollbar clear of the row's own
 * content, which one of the three copies of this had drifted without.
 */
export function ScrollRow({
  label,
  children,
  className,
}: {
  label?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex items-center gap-1.5 overflow-x-auto pb-0.5", className)}>
      {label && <span className="shrink-0 text-small text-faint-foreground">{label}</span>}
      {children}
    </div>
  );
}
