import { forwardRef, type SelectHTMLAttributes } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "../../lib/utils";

/** Native <select>, styled - preferred for reliability. */
export const Select = forwardRef<
  HTMLSelectElement,
  SelectHTMLAttributes<HTMLSelectElement>
>(({ className, children, ...props }, ref) => (
  <div className={cn("relative inline-flex w-full", className)}>
    <select
      ref={ref}
      className={cn(
        "h-9 w-full appearance-none rounded-lg border border-border bg-surface pl-3 pr-8 text-base text-foreground",
        "transition-colors duration-150 hover:border-border-strong",
        "focus:border-accent focus:outline-none focus-visible:outline-none",
        "disabled:cursor-not-allowed disabled:opacity-50",
      )}
      {...props}
    >
      {children}
    </select>
    <ChevronDown
      size={14}
      strokeWidth={1.75}
      className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-faint-foreground"
    />
  </div>
));
Select.displayName = "Select";
