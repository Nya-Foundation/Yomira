import type { ReactNode } from "react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";
import { Button } from "./button";
import { Modal, ModalHeader } from "./modal";

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  children: ReactNode;
  className?: string;
}

export function Dialog({ open, onOpenChange, title, children, className }: DialogProps) {
  return (
    <Modal open={open} onOpenChange={onOpenChange} label={typeof title === "string" ? title : undefined}
      className={cn("app-dialog w-full max-w-md rounded-xl border border-border bg-surface p-5", className)}>
      {title !== "" && (
        <ModalHeader title={title} onClose={() => onOpenChange(false)} className="mb-4" />
      )}
      {children}
    </Modal>
  );
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  confirmLabel?: string;
  onConfirm: () => void;
}) {
  const t = useT();
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={title} className="max-w-sm">
      {description && <p className="mb-4 text-base leading-6 text-muted-foreground">{description}</p>}
      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={() => onOpenChange(false)}>
          {t("common.cancel")}
        </Button>
        <Button
          variant="danger"
          onClick={() => {
            onConfirm();
            onOpenChange(false);
          }}
        >
          {confirmLabel ?? t("common.confirm")}
        </Button>
      </div>
    </Dialog>
  );
}
