import { useT } from "../../lib/i18n";
import { Button, GhostLink } from "../ui/button";

/**
 * Shown when the reader runs out of pages: reaching the bottom in continuous
 * mode, or trying to advance past the last page in either. Non-modal, so it
 * never stands between the reader and the page it is sitting on.
 */
export function EndOfGalleryCard({
  gid,
  token,
  pageCount,
  onRestart,
  onDismiss,
}: {
  gid: number;
  token: string;
  pageCount: number;
  onRestart: () => void;
  onDismiss: () => void;
}) {
  const t = useT();
  return (
    <div
      role="group"
      aria-label={t("reader.endTitle")}
      data-reader-end
      className="absolute bottom-4 left-1/2 z-30 w-[min(28rem,calc(100%-2rem))] -translate-x-1/2 rounded-lg border border-border bg-elevated/95 px-4 py-3 shadow-xl shadow-black/40"
    >
      <p className="text-base font-semibold text-foreground">{t("reader.endTitle")}</p>
      <p className="mt-0.5 text-small leading-5 text-muted-foreground">
        {t("reader.endDesc", { n: pageCount })}
      </p>
      <div className="mt-3 flex flex-wrap items-center gap-2">
        <GhostLink to={`/gallery/${gid}/${token}`}>{t("reader.endBackToGallery")}</GhostLink>
        <Button variant="outline" size="sm" onClick={onRestart}>
          {t("reader.endReadAgain")}
        </Button>
        <Button variant="ghost" size="sm" className="ml-auto" onClick={onDismiss}>
          {t("common.close")}
        </Button>
      </div>
    </div>
  );
}
