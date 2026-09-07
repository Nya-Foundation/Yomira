import { BookOpen, Pause, Play, Trash2 } from "lucide-react";
import { Link } from "react-router-dom";
import { CATEGORY_COLORS } from "../../lib/categories";
import { useT, type TKey } from "../../lib/i18n";
import { readProgress } from "../../lib/reading-progress";
import type { DownloadEntry, DownloadState } from "../../lib/types";
import { cn } from "../../lib/utils";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Select } from "../ui/select";

const STATE_VARIANTS: Record<DownloadState, "outline" | "accent" | "warning" | "danger" | "success"> = {
  none: "outline",
  queued: "accent",
  downloading: "accent",
  paused: "warning",
  done: "success",
  failed: "danger",
};

const STATE_KEYS: Record<DownloadState, TKey> = {
  none: "downloads.state.none",
  queued: "downloads.state.queued",
  downloading: "downloads.state.downloading",
  paused: "downloads.state.paused",
  done: "downloads.state.done",
  failed: "downloads.state.failed",
};

export function DownloadRow({
  entry: d,
  labels,
  busy,
  selectMode,
  selected,
  onToggleSelect,
  onStart,
  onPause,
  onRemove,
  onAssignLabel,
}: {
  entry: DownloadEntry;
  labels: string[];
  busy: boolean;
  selectMode: boolean;
  selected: boolean;
  onToggleSelect: () => void;
  onStart: () => void;
  onPause: () => void;
  onRemove: () => void;
  onAssignLabel: (label: string | null) => void;
}) {
  const t = useT();
  const readerPath = `/read/${d.gid}/${d.token}?source=download`;
  const savedPosition = readProgress(d.gid);
  const progress = d.pages > 0 ? Math.min(1, d.finishedPages / d.pages) : 0;
  const categoryColor = CATEGORY_COLORS[d.category as keyof typeof CATEGORY_COLORS];

  return (
    <div className="flex flex-wrap items-center gap-3 border-b border-border px-4 py-4">
      {selectMode && (
        <input
          type="checkbox"
          checked={selected}
          onChange={onToggleSelect}
          aria-label={d.title}
          className="h-4 w-4 shrink-0 accent-accent"
        />
      )}
      <Link
        to={d.finishedPages > 0 ? readerPath : `/gallery/${d.gid}/${d.token}`}
        className="block h-16 w-12 shrink-0 overflow-hidden rounded-md border border-border bg-surface-2"
      >
        {d.localThumb ? <img src={d.localThumb} alt={d.title} loading="lazy" className="h-full w-full object-cover" onError={(e) => { e.currentTarget.style.visibility = "hidden"; }} /> : <BookOpen aria-hidden className="m-auto h-full text-faint-foreground" size={20} />}
      </Link>

      <div className="min-w-0 flex-[1_1_12rem]">
        <div className="flex flex-wrap items-center gap-2">
          <Link
            to={d.finishedPages > 0 ? readerPath : `/gallery/${d.gid}/${d.token}`}
            className="truncate text-base font-medium hover:text-accent"
          >
            {d.title}
          </Link>
          {categoryColor && (
            <span
              aria-hidden
              className="h-1.5 w-1.5 shrink-0 rounded-full"
              style={{ backgroundColor: categoryColor }}
            />
          )}
          <Badge variant={STATE_VARIANTS[d.state]} className="shrink-0">
            {t(STATE_KEYS[d.state])}
          </Badge>
        </div>
        <div className="mt-1.5 flex items-center gap-2">
          <div className="h-1 w-full max-w-56 overflow-hidden rounded-full bg-surface-2">
            <div
              className={cn(
                "h-full rounded-full transition-[width] duration-300",
                d.state === "done" ? "bg-success" : "bg-accent",
              )}
              style={{ width: `${progress * 100}%` }}
            />
          </div>
          <span className="shrink-0 text-caption text-faint-foreground tabular-nums">
            {t("downloads.progress", { a: d.finishedPages, b: d.pages })}
          </span>
          {d.state === "failed" && d.error && (
            <span className="truncate text-caption text-danger">{d.error}</span>
          )}
        </div>
      </div>

      {d.finishedPages > 0 && <Link to={readerPath}
        className="inline-flex min-h-10 items-center gap-2 rounded-md border border-border px-3 text-small font-medium hover:bg-surface-2"
        aria-label={t("downloads.readOfflineTitle", { title: d.title })}>
        <BookOpen size={15} />
        {t(savedPosition > 0 ? "downloads.resumeOffline" : "downloads.readOffline")}
      </Link>}
      <div className="ml-auto flex shrink-0 items-center gap-1.5">
        <Select
          value={d.label ?? ""}
          onChange={(e) => onAssignLabel(e.target.value === "" ? null : e.target.value)}
          disabled={busy}
          aria-label={t("downloads.assignLabel")}
          className="w-32"
        >
          <option value="">{t("downloads.noLabel")}</option>
          {labels.map((label) => (
            <option key={label} value={label}>
              {label}
            </option>
          ))}
        </Select>
        {d.state === "downloading" || d.state === "queued" ? (
          <Button variant="outline" size="icon" onClick={onPause} disabled={busy} aria-label={t("downloads.pause")}>
            <Pause size={14} strokeWidth={1.75} />
          </Button>
        ) : (
          <Button
            variant="outline"
            size="icon"
            onClick={onStart}
            disabled={busy || d.state === "done"}
            aria-label={t("downloads.start")}
          >
            <Play size={14} strokeWidth={1.75} />
          </Button>
        )}
        <Button
          variant="ghost"
          size="icon"
          onClick={onRemove}
          disabled={busy}
          aria-label={t("downloads.remove")}
          className="text-muted-foreground hover:text-danger"
        >
          <Trash2 size={14} strokeWidth={1.75} />
        </Button>
      </div>
    </div>
  );
}
