import type { ReactNode } from "react";
import { X } from "lucide-react";
import { useT } from "../../lib/i18n";
import { Modal } from "./modal";

export function Sheet({ open, onOpenChange, title, children, width = 380 }: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  children: ReactNode;
  width?: number;
}) {
  const t = useT();
  return (
    <Modal open={open} onOpenChange={onOpenChange} label={typeof title === "string" ? title : undefined}
      className="app-sheet border-l border-border bg-surface">
      <div className="flex h-full flex-col" style={{ width: `min(${width}px, 100vw)` }}>
        <div className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-5 py-3">
          <h2 className="text-emphasis font-semibold tracking-tight">{title}</h2>
          <button onClick={() => onOpenChange(false)} aria-label={t("common.close")}
            className="flex size-9 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-surface-2 hover:text-foreground">
            <X size={18} strokeWidth={1.75} />
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-5 pb-[max(1.25rem,env(safe-area-inset-bottom))]">{children}</div>
      </div>
    </Modal>
  );
}
