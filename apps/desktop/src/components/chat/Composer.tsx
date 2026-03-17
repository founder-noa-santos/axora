import { ComposerPrimitive, ThreadPrimitive } from '@assistant-ui/react';
import { SendHorizontal, Square } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function Composer() {
  return (
    <ComposerPrimitive.Root className="focus-within:border-ring/20 flex w-full flex-wrap items-end rounded-xl border bg-card px-3 py-3 shadow-sm transition-colors ease-in">
      <ComposerPrimitive.Input
        rows={1}
        autoFocus
        placeholder="Ask AXORA anything..."
        className="placeholder:text-muted-foreground max-h-40 flex-grow resize-none border-none bg-transparent px-2 py-2 text-sm outline-none focus:ring-0 disabled:cursor-not-allowed"
      />
      <ComposerActions />
    </ComposerPrimitive.Root>
  );
}

function ComposerActions() {
  return (
    <div className="flex items-center gap-2">
      <ThreadPrimitive.If running={false}>
        <ComposerPrimitive.Send asChild>
          <Button
            size="icon"
            className="h-8 w-8 shrink-0"
            aria-label="Send message"
          >
            <SendHorizontal className="h-4 w-4" />
          </Button>
        </ComposerPrimitive.Send>
      </ThreadPrimitive.If>
      
      <ThreadPrimitive.If running>
        <ComposerPrimitive.Cancel asChild>
          <Button
            size="icon"
            variant="destructive"
            className="h-8 w-8 shrink-0"
            aria-label="Cancel generation"
          >
            <Square className="h-4 w-4 fill-current" />
          </Button>
        </ComposerPrimitive.Cancel>
      </ThreadPrimitive.If>
    </div>
  );
}
