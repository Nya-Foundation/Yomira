import { AlertTriangle } from "lucide-react";
import type { ReactNode } from "react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";
import { Button } from "./button";

export function ErrorState({
  title,
  description,
  onRetry,
  retryLabel,
  className,
  action,
}: {
  title?: string;
  description?: string;
  onRetry?: () => void;
  retryLabel?: string;
  className?: string;
  action?: ReactNode;
}) {
  const t = useT();
  return (
    <div
      role="alert"
      className={cn(
        "flex h-full w-full flex-col items-center justify-center gap-2 px-6 py-12 text-center",
        className,
      )}
    >
      <div className="mb-1 flex h-12 w-12 items-center justify-center rounded-full border border-danger/25 bg-danger/10">
        <AlertTriangle size={22} className="text-danger" strokeWidth={1.75} />
      </div>
      <p className="text-emphasis font-medium tracking-tight">{title ?? t("common.errorTitle")}</p>
      {description && (
        <p className="max-w-sm text-small leading-5 text-muted-foreground">{description}</p>
      )}
      <div className="mt-2 flex items-center gap-2">
        {onRetry && (
          <Button variant="outline" size="sm" onClick={onRetry}>
            {retryLabel ?? t("common.retry")}
          </Button>
        )}
        {action}
      </div>
    </div>
  );
}
