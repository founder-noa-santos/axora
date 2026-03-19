import * as React from "react";

import { cn } from "@/lib/utils";

const Input = React.forwardRef<HTMLInputElement, React.ComponentProps<"input">>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "flex h-10 w-full rounded-[12px] border border-white/8 bg-input/80 px-3 text-sm text-foreground shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring",
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = "Input";

export { Input };
