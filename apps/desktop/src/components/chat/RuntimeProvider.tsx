import { AssistantRuntimeProvider } from '@assistant-ui/react';
import { useOpenAICompatibleRuntime } from '@/hooks/useOpenAICompatibleRuntime';
import { ReactNode } from 'react';

interface RuntimeProviderProps {
  children: ReactNode;
}

export function RuntimeProvider({ children }: RuntimeProviderProps) {
  const runtime = useOpenAICompatibleRuntime();

  return (
    <AssistantRuntimeProvider runtime={runtime}>
      {children}
    </AssistantRuntimeProvider>
  );
}
