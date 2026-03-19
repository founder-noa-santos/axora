import * as SwitchPrimitives from "@radix-ui/react-switch";

import { cn } from "@/lib/utils";

function Switch({
  className,
  ...props
}: React.ComponentProps<typeof SwitchPrimitives.Root>) {
  return (
    <SwitchPrimitives.Root
      className={cn(
        "peer inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border border-white/8 bg-white/10 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring data-[state=checked]:bg-primary",
        className,
      )}
      {...props}
    >
      <SwitchPrimitives.Thumb className="pointer-events-none block h-5 w-5 translate-x-0.5 rounded-full bg-white shadow-lg ring-0 transition-transform data-[state=checked]:translate-x-[1.3rem]" />
    </SwitchPrimitives.Root>
  );
}

export { Switch };
