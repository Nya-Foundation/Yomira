import { useEffect, useState } from "react";
import { useT } from "../../lib/i18n";
import { Button } from "../ui/button";
import { Dialog } from "../ui/dialog";
import { Input } from "../ui/input";

export function NewLabelDialog({
  open,
  onOpenChange,
  pending,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  pending: boolean;
  onSubmit: (name: string) => void;
}) {
  const t = useT();
  const [name, setName] = useState("");
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t("downloads.newLabelTitle")}>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const v = name.trim();
          if (v) {
            onSubmit(v);
            setName("");
          }
        }}
        className="flex items-center gap-2"
      >
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t("downloads.newLabelPlaceholder")}
          autoFocus
        />
        <Button type="submit" size="md" disabled={pending || !name.trim()}>
          {t("common.add")}
        </Button>
      </form>
    </Dialog>
  );
}

export function RootDialog({
  open,
  onOpenChange,
  current,
  pending,
  onSubmit,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  current: string;
  pending: boolean;
  onSubmit: (path: string) => void;
}) {
  const t = useT();
  const [path, setPath] = useState(current);
  // Reset the draft each time the dialog opens with the freshest value.
  useEffect(() => {
    if (open) setPath(current);
  }, [open, current]);
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t("downloads.changeRootTitle")}>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const v = path.trim();
          if (v) onSubmit(v);
        }}
      >
        <Input
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder={t("downloads.rootPlaceholder")}
          className="mb-2 font-mono"
          autoFocus
        />
        <p className="mb-4 text-caption leading-5 text-faint-foreground">{t("downloads.rootHint")}</p>
        <div className="flex justify-end gap-2">
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" size="sm" disabled={pending || !path.trim()}>
            {t("common.apply")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
