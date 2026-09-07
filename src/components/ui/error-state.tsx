import { AlertTriangle } from "lucide-react";
import type { ReactNode } from "react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

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
          <button
            onClick={onRetry}
            className="h-8 rounded-md border border-danger/30 bg-danger/10 px-3 text-small font-medium text-danger transition-colors hover:bg-danger/20"
          >
            {retryLabel ?? t("common.retry")}
          </button>
        )}
        {action}
      </div>
    </div>
  );
}
