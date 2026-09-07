import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "../../lib/utils";

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: {
  icon?: LucideIcon;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex h-full w-full flex-col items-center justify-center gap-2 px-6 py-16 text-center",
        className,
      )}
    >
      {Icon && (
        <div className="mb-1 flex h-12 w-12 items-center justify-center rounded-full border border-border bg-surface">
          <Icon size={22} className="text-faint-foreground" strokeWidth={1.75} />
        </div>
      )}
      <p className="text-emphasis font-medium tracking-tight">{title}</p>
      {description && (
        <p className="max-w-sm text-small leading-5 text-muted-foreground">{description}</p>
      )}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}
