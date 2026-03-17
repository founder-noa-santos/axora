import { Thread } from '@/components/chat/Thread';
import { RuntimeProvider } from '@/components/chat/RuntimeProvider';

export function ChatPanel() {
  return (
    <div className="flex h-full flex-col">
      <RuntimeProvider>
        <Thread />
      </RuntimeProvider>
    </div>
  );
}
