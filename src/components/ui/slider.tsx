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
      style={{
        background: `linear-gradient(to right, var(--color-accent) ${pct}%, var(--color-surface-2) ${pct}%)`,
      }}
      className={cn(
        "h-1 w-full cursor-pointer appearance-none rounded-full",
        "[&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:appearance-none",
        "[&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-accent",
        "[&::-moz-range-thumb]:h-3 [&::-moz-range-thumb]:w-3 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-accent",
        className,
      )}
      {...props}
    />
  );
});
Slider.displayName = "Slider";
