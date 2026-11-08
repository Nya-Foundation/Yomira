import { useT } from "../../lib/i18n";
import { Button } from "../ui/button";
import { Dialog } from "../ui/dialog";
import { Spinner } from "../ui/spinner";
import { Textarea } from "../ui/textarea";

/**
 * Dialog for editing the private favorite note attached to a gallery.
 *
 * Saving sends the whole note, so editing one that failed to load would post
 * an empty string over whatever is stored. Until the current note is in hand
 * the editor is withheld and saving is refused.
 */
export function FavoriteNoteDialog({
  open,
  onOpenChange,
  draft,
  onDraftChange,
  status,
  onRetry,
  saving,
  onSave,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  draft: string;
  onDraftChange: (value: string) => void;
  status: "loading" | "error" | "ready";
  onRetry: () => void;
  saving: boolean;
  onSave: () => void;
}) {
  const t = useT();
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t("detail.favNoteTitle")}>
      {status === "loading" && (
        <p role="status" className="flex items-center gap-2 py-6 text-base text-muted-foreground">
          <Spinner size={16} aria-hidden />
          {t("detail.favNoteLoading")}
        </p>
      )}
      {status === "error" && (
        <p role="alert" className="rounded-md border border-danger/25 bg-danger/5 px-3 py-2 text-base leading-6 text-danger">
          {t("detail.favNoteLoadFailed")}
        </p>
      )}
      {status === "ready" && (
        <Textarea
          value={draft}
          onChange={(e) => onDraftChange(e.target.value)}
          placeholder={t("detail.favNotePlaceholder")}
          autoFocus
        />
      )}
      <div className="mt-3 flex justify-end gap-2">
        <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
          {t("common.cancel")}
        </Button>
        {status === "error" ? (
          <Button size="sm" onClick={onRetry}>
            {t("common.retry")}
          </Button>
        ) : (
          <Button size="sm" disabled={saving || status !== "ready"} onClick={onSave}>
            {t("common.apply")}
          </Button>
        )}
      </div>
    </Dialog>
  );
}
