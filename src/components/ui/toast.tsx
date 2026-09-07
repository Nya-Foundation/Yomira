import { CheckCircle2, AlertCircle, Info, X } from "lucide-react";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useToasts, type ToastKind } from "../../stores/toast";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

const icons: Record<ToastKind, typeof Info> = {
  success: CheckCircle2,
  error: AlertCircle,
  info: Info,
};

const colors: Record<ToastKind, string> = {
  success: "text-success",
  error: "text-danger",
  info: "text-muted-foreground",
};

export function ToastViewport() {
  const { toasts, dismiss } = useToasts();
  const tr = useT();
  const [portalTarget, setPortalTarget] = useState<Element>(document.body);
  useEffect(() => {
    const updateTarget = () => {
      const dialogs = document.querySelectorAll("dialog[open]");
      setPortalTarget(dialogs.item(dialogs.length - 1) ?? document.body);
    };
    // Native dialogs live above every document z-index. Keep feedback visible
    // in that same layer, and move it back when the dialog closes.
    const observer = new MutationObserver(updateTarget);
    observer.observe(document.body, { subtree: true, attributes: true, attributeFilter: ["open"] });
    updateTarget();
    return () => observer.disconnect();
  }, []);
  return createPortal(
    <div
      aria-live="polite"
      className="pointer-events-none fixed bottom-[calc(5rem+env(safe-area-inset-bottom))] right-4 z-[100] flex w-80 max-w-[calc(100vw-2rem)] md:bottom-4 flex-col gap-2"
    >
      {toasts.map((t) => {
        const Icon = icons[t.kind];
        return (
          <div
            key={t.id}
            className="pointer-events-auto flex items-start gap-2.5 rounded-lg border border-border bg-elevated p-3 shadow-lg shadow-black/40 animate-[toast-in_180ms_cubic-bezier(0.16,1,0.3,1)]"
          >
            <Icon size={16} strokeWidth={1.75} className={cn("mt-0.5 shrink-0", colors[t.kind])} />
            <div className="min-w-0 flex-1">
              <p className="text-base font-medium leading-5">{t.title}</p>
              {t.description && (
                <p className="mt-0.5 text-caption leading-4 text-muted-foreground">{t.description}</p>
              )}
            </div>
            <button
              onClick={() => dismiss(t.id)}
              aria-label={tr("common.dismiss")}
              className="shrink-0 rounded p-0.5 text-faint-foreground transition-colors hover:text-foreground"
            >
              <X size={14} strokeWidth={1.75} />
            </button>
          </div>
        );
      })}
    </div>,
    portalTarget,
  );
}
