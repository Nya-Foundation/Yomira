import { Loader2 } from "lucide-react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

export function Spinner({ className, size = 18 }: { className?: string; size?: number }) {
  const t = useT();
  return (
    <Loader2
      size={size}
      className={cn("animate-spin text-muted-foreground", className)}
      aria-label={t("common.loading")}
      role="status"
    />
  );
}
