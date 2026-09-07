import { Link } from "react-router-dom";
import { CachedImage } from "../ui/cached-image";
import { RatingHearts } from "./rating-hearts";
import { categoryLabel, useT } from "../../lib/i18n";
import type { GalleryInfo } from "../../lib/types";
import { formatRelativeTime } from "../../lib/utils";

export function GalleryCard({ gallery, detail = false }: { gallery: GalleryInfo; detail?: boolean }) {
  useT();
  if (detail) return (
    <Link to={`/gallery/${gallery.gid}/${gallery.token}`} data-gallery-view="detail"
      className="group flex gap-4 border-b border-border py-3 transition-colors hover:bg-surface">
      <div className="h-32 w-24 shrink-0 overflow-hidden rounded-lg bg-surface-2">
        <CachedImage url={gallery.thumb} alt={gallery.title} className="h-full w-full object-cover" />
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-2 py-1">
        <p className="line-clamp-2 text-base font-medium leading-5 group-hover:text-accent">{gallery.title}</p>
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-caption text-muted-foreground">
          <span>{categoryLabel(gallery.category)}</span><span>{gallery.pages}p</span>
          {gallery.language && <span>{gallery.language}</span>}
        </div>
        <RatingHearts value={gallery.rating} />
        <p className="truncate text-small text-muted-foreground">{gallery.uploader}</p>
        <p className="text-caption text-faint-foreground">{formatRelativeTime(gallery.posted)}</p>
      </div>
    </Link>
  );
  return (
    <Link
      to={`/gallery/${gallery.gid}/${gallery.token}`}
      data-density
      className="group flex h-full flex-col rounded-lg transition-colors hover:bg-surface"
    >
      <div className="relative aspect-[3/4] overflow-hidden rounded-lg bg-surface-2">
        <CachedImage
          url={gallery.thumb}
          alt={gallery.title}
          className="h-full w-full object-cover"
        />
      </div>
      <div className="flex flex-col gap-1 px-1 py-[var(--density-pad)]">
        <p className="line-clamp-2 min-h-10 text-base font-medium leading-5 text-foreground group-hover:text-accent">
          {gallery.title}
        </p>
        <div className="flex min-w-0 items-center justify-between gap-2 text-caption text-muted-foreground">
          <span className="truncate">{categoryLabel(gallery.category)}</span>
          <span className="shrink-0 tabular-nums">{gallery.pages}p</span>
        </div>
        <div className="flex min-w-0 items-center gap-2">
          <RatingHearts value={gallery.rating} />
        </div>
        <p className="truncate text-small text-faint-foreground">
          {gallery.uploader} · {formatRelativeTime(gallery.posted)}
        </p>
      </div>
    </Link>
  );
}

export function GalleryCardSkeleton() {
  return (
    <div
      data-density
      className="flex flex-col overflow-hidden rounded-lg border border-border bg-surface"
    >
      <div className="aspect-[3/4] animate-pulse bg-surface-2" />
      <div className="space-y-1.5 p-[var(--density-pad)]">
        <div className="h-3.5 w-4/5 animate-pulse rounded bg-surface-2" />
        <div className="h-3 w-3/5 animate-pulse rounded bg-surface-2" />
      </div>
    </div>
  );
}
