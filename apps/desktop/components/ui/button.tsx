import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-[12px] text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default:
          "bg-white/12 text-foreground shadow-[inset_0_1px_0_rgba(255,255,255,0.08)] hover:bg-white/16",
        primary:
          "bg-primary text-primary-foreground shadow-[0_10px_30px_rgba(137,159,196,0.18)] hover:bg-primary/90",
        ghost: "text-muted-foreground hover:bg-white/6 hover:text-foreground",
        outline:
          "border border-white/10 bg-transparent text-foreground hover:bg-white/6",
      },
      size: {
        default: "h-10 px-4",
        sm: "h-8 rounded-[10px] px-3 text-[13px]",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";

export { Button, buttonVariants };
