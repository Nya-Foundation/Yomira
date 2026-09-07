import { useQuery } from "@tanstack/react-query";
import { HardDrive } from "lucide-react";
import { getQuota } from "../../lib/api";
import { useT } from "../../lib/i18n";
import { formatCount } from "../../lib/utils";

function formatReset(t: ReturnType<typeof useT>, seconds: number): string {
  if (seconds < 3600) return t("quota.minutes", { n: Math.max(1, Math.ceil(seconds / 60)) });
  if (seconds < 86400) return t("quota.hours", { n: Math.ceil(seconds / 3600) });
  return t("quota.days", { n: Math.ceil(seconds / 86400) });
}

/**
 * Slim image-quota strip for the Home header area. Renders nothing while
 * the backend reports the quota as unknown (-1 / null), so it stays out of
 * the way for guests and unsupported backends.
 */
export function QuotaBar() {
  const t = useT();
  const { data: quota } = useQuery({
    queryKey: ["quota"],
    queryFn: getQuota,
    refetchInterval: 300_000,
  });

  if (!quota || quota.imagesUsed == null || quota.imagesUsed < 0) return null;
  const total = quota.imagesTotal != null && quota.imagesTotal > 0 ? quota.imagesTotal : null;
  const pct = total ? Math.min(100, (quota.imagesUsed / total) * 100) : null;

  return (
    <div className="flex shrink-0 items-center gap-2.5 border-b border-border px-4 py-1.5">
      <HardDrive size={13} strokeWidth={1.75} className="shrink-0 text-faint-foreground" />
      <span className="shrink-0 text-caption text-muted-foreground">
        {t("quota.images", { a: formatCount(quota.imagesUsed), b: total != null ? formatCount(total) : "?" })}
      </span>
      <div className="h-1 w-28 overflow-hidden rounded-full bg-surface-2">
        {pct != null && (
          <div
            className={`h-full rounded-full ${pct >= 90 ? "bg-danger" : pct >= 70 ? "bg-warning" : "bg-accent"}`}
            style={{ width: `${pct}%` }}
          />
        )}
      </div>
      {quota.imageResetSeconds != null && quota.imageResetSeconds > 0 && (
        <span className="ml-auto shrink-0 text-caption text-faint-foreground">
          {t("quota.resetsIn", { t: formatReset(t, quota.imageResetSeconds) })}
        </span>
      )}
    </div>
  );
}
