import { LayoutGrid, List } from "lucide-react";
import { useT } from "../../lib/i18n";
import { useSettings } from "../../lib/settings";
import { Button } from "../ui/button";

/**
 * Thumbnail / detailed-list switch. It lives in each page's existing toolbar
 * rather than in a band of its own: a once-per-user preference was costing a
 * full-width row above the grid, which on a tablet in landscape was a
 * meaningful share of the covers visible at all.
 */
export function GalleryViewToggle() {
  const t = useT();
  const view = useSettings((s) => s.settings.galleryView);
  const update = useSettings((s) => s.update);
  return (
    <div className="flex shrink-0 items-center gap-1" role="group" aria-label={t("list.galleryView")}>
      {(["thumbnail", "detail"] as const).map((mode) => (
        <Button
          key={mode}
          variant={view === mode ? "subtle" : "ghost"}
          size="md"
          aria-pressed={view === mode}
          onClick={() => update({ galleryView: mode })}
          aria-label={t(mode === "thumbnail" ? "list.thumbnailView" : "list.detailView")}
        >
          {mode === "thumbnail" ? <LayoutGrid size={14} /> : <List size={14} />}
        </Button>
      ))}
    </div>
  );
}
