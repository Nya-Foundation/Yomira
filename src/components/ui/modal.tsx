import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

/** Native modal supplies focus containment, background inertness and stacked Escape handling. */
export function Modal({ open, onOpenChange, label, className, children }: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  label?: string;
  className?: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const onChange = useRef(onOpenChange);
  onChange.current = onOpenChange;

  useEffect(() => {
    if (!open) return;
    const dialog = ref.current;
    const trigger = document.activeElement;
    dialog?.showModal();
    return () => {
      dialog?.close();
      if (trigger instanceof HTMLElement && trigger.isConnected) trigger.focus();
    };
  }, [open]);

  return createPortal(
    <dialog
      ref={ref}
      aria-label={label}
      className={cn("app-modal text-foreground", className)}
      onCancel={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onChange.current(false);
      }}
      onKeyDown={(event) => {
        if (event.key !== "Tab") return;
        const elements = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(
          'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
        )).filter((element) => element.getClientRects().length > 0);
        const first = elements[0];
        const last = elements[elements.length - 1];
        if (!first) { event.preventDefault(); return; }
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }}
      onClick={(event) => {
        if (event.target !== event.currentTarget) return;
        const rect = event.currentTarget.getBoundingClientRect();
        if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) {
          onChange.current(false);
        }
      }}
    >
      {open && children}
    </dialog>,
    document.body,
  );
}

/** Title and close button, shared by the dialog and the sheet. */
export function ModalHeader({
  title,
  onClose,
  className,
}: {
  title: ReactNode;
  onClose: () => void;
  className?: string;
}) {
  const t = useT();
  return (
    <div className={cn("flex items-center justify-between gap-3", className)}>
      <h2 className="text-emphasis font-semibold tracking-tight">{title}</h2>
      <button
        onClick={onClose}
        aria-label={t("common.close")}
        className="flex size-9 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-surface-2 hover:text-foreground"
      >
        <X size={18} strokeWidth={1.75} />
      </button>
    </div>
  );
}
