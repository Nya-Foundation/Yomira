import type { ReactNode } from "react";
import { Modal, ModalHeader } from "./modal";

export function Sheet({ open, onOpenChange, title, children, width = 380 }: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  children: ReactNode;
  width?: number;
}) {
  return (
    <Modal open={open} onOpenChange={onOpenChange} label={typeof title === "string" ? title : undefined}
      className="app-sheet border-l border-border bg-surface">
      <div className="flex h-full flex-col" style={{ width: `min(${width}px, 100vw)` }}>
        <ModalHeader
          title={title}
          onClose={() => onOpenChange(false)}
          className="shrink-0 border-b border-border px-5 py-3"
        />
        <div className="min-h-0 flex-1 overflow-y-auto p-5 pb-[max(1.25rem,env(safe-area-inset-bottom))]">{children}</div>
      </div>
    </Modal>
  );
}
