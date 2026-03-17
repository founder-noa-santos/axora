import { MessagePrimitive } from '@assistant-ui/react';
import { Bot } from 'lucide-react';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { MarkdownText } from './MarkdownText';
import { ActionBar } from './ActionBar';

export function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="grid w-full max-w-[var(--thread-max-width)] grid-cols-[auto_1fr] gap-3 py-4">
      <Avatar className="h-8 w-8">
        <AvatarFallback className="bg-secondary text-secondary-foreground">
          <Bot className="h-4 w-4" />
        </AvatarFallback>
      </Avatar>

      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold">AXORA</span>
        </div>
        <div className="text-foreground max-w-[calc(var(--thread-max-width)*0.9)] break-words leading-7">
          <MessagePrimitive.Content
            components={{ Text: MarkdownText }}
          />
        </div>
        <ActionBar />
      </div>
    </MessagePrimitive.Root>
  );
}
