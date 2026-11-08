import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "../../lib/utils";

/** Native range input, styled thin. */
export const Slider = forwardRef<
  HTMLInputElement,
  InputHTMLAttributes<HTMLInputElement> & { min: number; max: number }
>(({ className, min, max, value, ...props }, ref) => {
  const pct =
    max > min
      ? (((Number(value ?? min) - min) / (max - min)) * 100).toFixed(1)
      : "0";
  return (
    <input
      ref={ref}
      type="range"
      min={min}
      max={max}
      value={value}
      style={{ "--slider-pct": `${pct}%` } as React.CSSProperties}
      className={cn("app-slider", className)}
      {...props}
    />
  );
});
Slider.displayName = "Slider";
