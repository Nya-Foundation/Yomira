import type { ReactNode } from "react";
import { cn } from "../../lib/utils";

export const tabButtonId = (base: string, value: string) => `${base}-tab-${value}`;
export const tabPanelId = (base: string, value: string) => `${base}-panel-${value}`;

export interface TabDef {
  value: string;
  label: ReactNode;
}

export function Tabs({
  tabs,
  value,
  onValueChange,
  idBase,
  className,
}: {
  tabs: TabDef[];
  value: string;
  onValueChange: (value: string) => void;
  /** Ties each tab to its panel; the panel must carry `tabPanelId(idBase, value)`. */
  idBase: string;
  className?: string;
}) {
  return (
    <div
      role="tablist"
      className={cn(
        "flex max-w-full items-center gap-1 overflow-x-auto rounded-lg border border-border bg-surface p-1",
        className,
      )}
    >
      {tabs.map((t) => (
        <button
          key={t.value}
          role="tab"
          id={tabButtonId(idBase, t.value)}
          // Only the selected panel is mounted, so pointing an inactive tab at
          // it would leave a dangling IDREF.
          aria-controls={value === t.value ? tabPanelId(idBase, t.value) : undefined}
          aria-selected={value === t.value}
          tabIndex={value === t.value ? 0 : -1}
          onKeyDown={(event) => {
            const current = tabs.findIndex((tab) => tab.value === t.value);
            const next = event.key === "ArrowRight" ? (current + 1) % tabs.length
              : event.key === "ArrowLeft" ? (current - 1 + tabs.length) % tabs.length
              : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : -1;
            if (next < 0) return;
            event.preventDefault();
            onValueChange(tabs[next].value);
            event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
          }}
          onClick={() => onValueChange(t.value)}
          className={cn(
            "h-9 shrink-0 whitespace-nowrap rounded-md px-3 text-small transition-colors duration-150",
            value === t.value
              ? "bg-surface-2 text-foreground"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}
