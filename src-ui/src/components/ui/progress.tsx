import * as React from "react"
import * as ProgressPrimitive from "@radix-ui/react-progress"

import { cn } from "@/lib/utils"

const Progress = React.forwardRef<
  React.ElementRef<typeof ProgressPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof ProgressPrimitive.Root>
>(({ className, value, ...props }, ref) => {
  // Radix Progress expects a finite number; clamp for safety so we always render something sane.
  const safeValue =
    typeof value === "number" && Number.isFinite(value)
      ? Math.min(100, Math.max(0, value))
      : 0

  return (
    <ProgressPrimitive.Root
      ref={ref}
      value={safeValue}
      className={cn(
        // Border + slightly muted track helps keep it visible at small heights (e.g. `h-2`)
        "relative h-4 w-full overflow-hidden rounded-full border border-border bg-secondary/60",
        className
      )}
      {...props}
    >
      <ProgressPrimitive.Indicator
        className="h-full bg-primary transition-all"
        style={{ width: `${safeValue}%` }}
      />
    </ProgressPrimitive.Root>
  )
})
Progress.displayName = ProgressPrimitive.Root.displayName

export { Progress }

