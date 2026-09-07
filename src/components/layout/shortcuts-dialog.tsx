import { Dialog } from "../ui/dialog";
import { useT, type TKey } from "../../lib/i18n";

const SHORTCUTS: [string, TKey][] = [
  ["Alt + ← / →", "shortcuts.history"],
  ["Ctrl/⌘ + + / −", "shortcuts.zoom"],
  ["Ctrl/⌘ + scroll", "shortcuts.zoom"],
  ["Ctrl/⌘ + 0", "shortcuts.zoomReset"],
  ["Ctrl/⌘ + K", "shortcuts.palette"],
  ["/", "shortcuts.focusSearch"],
  ["?", "shortcuts.help"],
  ["ArrowLeft / ArrowRight or J / K", "shortcuts.readerNav"],
  ["Space", "shortcuts.spaceNext"],
  ["Esc", "shortcuts.esc"],
  ["Enter", "shortcuts.enter"],
];

export function ShortcutsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT();
  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t("shortcuts.title")}>
      <ul className="space-y-1.5">
        {SHORTCUTS.map(([keys, descKey]) => (
          <li key={keys} className="flex items-center justify-between gap-4">
            <span className="text-base text-muted-foreground">{t(descKey)}</span>
            <kbd className="shrink-0 rounded border border-border bg-surface-2 px-1.5 py-0.5 font-mono text-caption text-foreground">
              {keys}
            </kbd>
          </li>
        ))}
      </ul>
    </Dialog>
  );
}
