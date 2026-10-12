import { cn } from "@/lib/cn";

/** Horizontal usage bar; turns amber above 75% and red above 90%. */
export function Meter({ percent, className }: { percent: number; className?: string }) {
  const value = Math.max(0, Math.min(100, percent));
  const tone = value > 90 ? "bg-down" : value > 75 ? "bg-degraded" : "bg-accent";
  return (
    <div className={cn("h-2 overflow-hidden rounded-full bg-fg/10", className)}>
      <div className={cn("h-full transition-all", tone)} style={{ width: `${value}%` }} />
    </div>
  );
}
