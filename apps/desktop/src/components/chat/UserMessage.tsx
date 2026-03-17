import { MessagePrimitive } from '@assistant-ui/react';
import { User } from 'lucide-react';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';

export function UserMessage() {
  return (
    <MessagePrimitive.Root className="grid w-full max-w-[var(--thread-max-width)] grid-cols-[auto_1fr] gap-3 py-4">
      <Avatar className="h-8 w-8">
        <AvatarFallback className="bg-primary text-primary-foreground">
          <User className="h-4 w-4" />
        </AvatarFallback>
      </Avatar>
      
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold">You</span>
        </div>
        <div className="text-foreground break-words leading-7">
          <MessagePrimitive.Content />
        </div>
      </div>
    </MessagePrimitive.Root>
  );
}
