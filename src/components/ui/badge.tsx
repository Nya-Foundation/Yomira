import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/utils";

type Variant = "default" | "accent" | "outline" | "success" | "warning" | "danger";

const variants: Record<Variant, string> = {
  default: "bg-surface-2 text-muted-foreground border border-border",
  accent: "bg-accent/15 text-accent border border-accent/25",
  outline: "bg-transparent text-muted-foreground border border-border",
  success: "bg-success/10 text-success border border-success/25",
  warning: "bg-warning/10 text-warning border border-warning/25",
  danger: "bg-danger/10 text-danger border border-danger/25",
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: Variant;
}

export function Badge({ className, variant = "default", ...props }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded px-1.5 py-px text-caption font-medium leading-4 whitespace-nowrap",
        variants[variant],
        className,
      )}
      {...props}
    />
  );
}

export function CategoryBadge({
  color,
  children,
}: {
  color: string;
  children: ReactNode;
}) {
  return (
    <span
      className="inline-flex items-center rounded px-1.5 py-px text-caption font-semibold leading-4 whitespace-nowrap"
      style={{ backgroundColor: `${color}2b`, color, border: `1px solid ${color}55` }}
    >
      {children}
    </span>
  );
}
