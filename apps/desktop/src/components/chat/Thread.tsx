import { ThreadPrimitive } from '@assistant-ui/react';
import { Composer } from './Composer';
import { UserMessage } from './UserMessage';
import { AssistantMessage } from './AssistantMessage';
import { WelcomeScreen } from './WelcomeScreen';

export function Thread() {
  return (
    <ThreadPrimitive.Root
      className="bg-background flex h-full flex-col overflow-hidden"
      style={{ ['--thread-max-width' as string]: '48rem' }}
    >
      <ThreadPrimitive.Viewport className="flex h-full flex-col items-center overflow-y-scroll scroll-smooth bg-inherit px-4 pt-8">
        {/* Empty state */}
        <ThreadPrimitive.If empty>
          <WelcomeScreen />
        </ThreadPrimitive.If>

        {/* Message list */}
        <ThreadPrimitive.Messages
          components={{
            UserMessage: UserMessage,
            AssistantMessage: AssistantMessage,
          }}
        />

        {/* Spacer for scroll */}
        <ThreadPrimitive.If empty={false}>
          <div className="min-h-8 flex-grow" />
        </ThreadPrimitive.If>

        {/* Composer at bottom */}
        <div className="sticky bottom-0 mt-3 flex w-full max-w-[var(--thread-max-width)] flex-col items-center justify-end rounded-t-lg bg-inherit pb-4">
          <ThreadPrimitive.ScrollToBottom />
          <Composer />
        </div>
      </ThreadPrimitive.Viewport>
    </ThreadPrimitive.Root>
  );
}
