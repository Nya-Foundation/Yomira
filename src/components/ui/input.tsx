import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "../../lib/utils";
import { FIELD_BASE } from "./field";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(FIELD_BASE, "h-9 px-3", className)}
      {...props}
    />
  ),
);
Input.displayName = "Input";
