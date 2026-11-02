import * as React from "react";
import { ChevronUpDownIcon } from "@heroicons/react/20/solid";
import { cn } from "@/lib/utils";

/**
 * A native select dressed in the app's field style.
 *
 * Native keeps the OS picker, keyboard behaviour and accessibility for free;
 * only the closed control is restyled so it matches the text inputs beside it.
 */
export const Select = React.forwardRef<HTMLSelectElement, React.SelectHTMLAttributes<HTMLSelectElement>>(
    ({ className, children, ...props }, ref) => (
        <div className={cn("relative", className)}>
            <select
                ref={ref}
                className={cn(
                    "w-full h-9 appearance-none rounded-md border border-input bg-background pl-3 pr-8 text-sm",
                    "transition-colors hover:border-muted-foreground/40",
                    "focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-ring",
                    "disabled:opacity-50 disabled:pointer-events-none"
                )}
                {...props}
            >
                {children}
            </select>
            <ChevronUpDownIcon className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        </div>
    )
);
Select.displayName = "Select";
