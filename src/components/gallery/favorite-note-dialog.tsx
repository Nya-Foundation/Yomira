import { useT } from "../../lib/i18n";
import { Button } from "../ui/button";
import { Dialog } from "../ui/dialog";
import { Textarea } from "../ui/textarea";

/** Dialog for editing the private favorite note attached to a gallery. */
export function FavoriteNoteDialog({
  open,
  onOpenChange,
  draft,
  onDraftChange,
  saving,
  onSave,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  draft: string;
  onDraftChange: (value: string) => void;
  saving: boolean;
  onSave: () => void;
}) {
  const t = useT();
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t("detail.favNoteTitle")}>
      <Textarea
        value={draft}
        onChange={(e) => onDraftChange(e.target.value)}
        placeholder={t("detail.favNotePlaceholder")}
        autoFocus
      />
      <div className="mt-3 flex justify-end gap-2">
        <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
          {t("common.cancel")}
        </Button>
        <Button size="sm" disabled={saving} onClick={onSave}>
          {t("common.apply")}
        </Button>
      </div>
    </Dialog>
  );
}
