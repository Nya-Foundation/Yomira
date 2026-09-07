import { forwardRef, type ButtonHTMLAttributes } from "react";
import { Link, type LinkProps } from "react-router-dom";
import { cn } from "../../lib/utils";

type Variant = "default" | "outline" | "ghost" | "ghost-link" | "subtle";
type Size = "sm" | "md" | "icon";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variants: Record<Variant, string> = {
  default:
    "bg-accent text-accent-foreground hover:bg-accent/85 border border-transparent",
  outline:
    "border border-border bg-transparent text-muted-foreground hover:text-foreground hover:border-border-strong hover:bg-surface-2",
  ghost: "text-muted-foreground hover:text-foreground hover:bg-surface-2",
  "ghost-link":
    "inline-flex h-8 items-center rounded-md border border-border px-3 text-small text-muted-foreground transition-colors hover:bg-surface-2 hover:text-foreground",
  subtle: "bg-surface-2 text-foreground hover:bg-overlay border border-border",
};

const sizes: Record<Size, string> = {
  sm: "h-7 px-2.5 text-small rounded-md",
  md: "h-9 px-3.5 text-base rounded-md",
  icon: "h-8 w-8 rounded-md",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "md", type = "button", ...props }, ref) => (
    <button
      ref={ref}
      type={type}
      className={cn(
        "inline-flex items-center justify-center gap-1.5 font-medium whitespace-nowrap",
        "transition-[background-color,color,border-color] duration-150 select-none",
        "disabled:pointer-events-none disabled:opacity-50",
        variants[variant],
        sizes[size],
        className,
      )}
      {...props}
    />
  ),
);
Button.displayName = "Button";

/** Router Link wearing the ghost-link Button look (navigation actions in empty/error states). */
export function GhostLink({ className, ...props }: LinkProps) {
  return <Link className={cn(variants["ghost-link"], className)} {...props} />;
}
