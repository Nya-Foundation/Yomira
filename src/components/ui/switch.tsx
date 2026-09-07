import { forwardRef } from "react";
import { cn } from "../../lib/utils";

export interface SwitchProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  "aria-label"?: string;
  className?: string;
}

/** Checkbox-based switch. */
export const Switch = forwardRef<HTMLButtonElement, SwitchProps>(
  ({ checked, onCheckedChange, disabled, className, ...aria }, ref) => (
    <button
      ref={ref}
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onCheckedChange(!checked)}
      className={cn(
        "relative h-5 w-9 shrink-0 rounded-full border transition-colors duration-200",
        checked ? "border-accent/40 bg-accent/30" : "border-border bg-surface-2",
        "disabled:pointer-events-none disabled:opacity-50",
        className,
      )}
      {...aria}
    >
      <span
        className={cn(
          "absolute top-1/2 h-3.5 w-3.5 -translate-y-1/2 rounded-full transition-[left,background-color] duration-200",
          checked ? "left-[18px] bg-accent" : "left-[3px] bg-muted-foreground",
        )}
      />
    </button>
  ),
);
Switch.displayName = "Switch";
