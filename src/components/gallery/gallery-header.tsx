import type { ReactNode } from "react";
import type { GalleryDetail } from "../../lib/types";
import { categoryLabel, useT } from "../../lib/i18n";
import { CATEGORY_COLORS } from "../../lib/categories";
import { formatBytes, formatCount, formatRelativeTime } from "../../lib/utils";
import { CachedImage } from "../ui/cached-image";
import { CategoryBadge } from "../ui/badge";

/**
 * Cover-led header of the gallery detail page: sticky cover, category badge,
 * title (plus Japanese title) and the meta grid. `children` continue the
 * info column (rating + action buttons).
 */
export function GalleryHeader({ d, children }: { d: GalleryDetail; children?: ReactNode }) {
  const t = useT();
  return (
    <div className="flex flex-col gap-6 sm:flex-row sm:gap-8">
      {/* cover column (sticky on large screens) */}
      <div className="w-32 shrink-0 sm:w-44 lg:w-60">
        <div className="lg:sticky lg:top-6">
          <div className="aspect-[3/4] overflow-hidden rounded-lg border border-border bg-surface-2">
            <CachedImage url={d.thumb} alt={d.title} className="h-full w-full object-cover" />
          </div>
        </div>
      </div>

      <div className="min-w-0 flex-1 space-y-4">
        <div className="flex flex-wrap items-center gap-2">
          <CategoryBadge color={CATEGORY_COLORS[d.category]}>{categoryLabel(d.category)}</CategoryBadge>
          <span className="text-caption text-faint-foreground">
            {formatRelativeTime(d.posted)}
          </span>
        </div>
        <div>
          <h2 className="text-title font-semibold leading-7 tracking-tight">{d.title}</h2>
          {d.titleJpn && (
            <p className="mt-1 text-small text-muted-foreground">{d.titleJpn}</p>
          )}
        </div>

        {/* meta grid */}
        <dl className="grid max-w-lg grid-cols-2 gap-x-6 gap-y-2 text-base sm:grid-cols-3">
          {[
            [t("detail.uploader"), d.uploader],
            [t("detail.language"), d.language],
            [t("detail.pages"), `${d.pages}`],
            [t("detail.size"), formatBytes(d.size)],
            [t("detail.favorited"), formatCount(d.favorited)],
            [t("detail.rated"), formatCount(d.ratedCount)],
          ].map(([k, v]) => (
            <div key={k}>
              <dt className="text-caption text-faint-foreground">{k}</dt>
              <dd className="truncate font-medium tabular-nums">{v}</dd>
            </div>
          ))}
        </dl>

        {children}
      </div>
    </div>
  );
}
