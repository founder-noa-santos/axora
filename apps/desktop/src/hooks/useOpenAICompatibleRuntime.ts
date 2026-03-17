import { useMemo } from 'react';
import { useChatRuntime, AssistantChatTransport } from '@assistant-ui/react-ai-sdk';
import { useSettingsStore } from '@/store/settings-store';
import type { AppSettings } from '@/types/settings';

export function useOpenAICompatibleRuntime() {
  const { settings } = useSettingsStore();
  
  const runtime = useChatRuntime({
    transport: new AssistantChatTransport({
      api: getApiEndpoint(settings),
      headers: getApiHeaders(settings),
    }),
  });

  return runtime;
}

function getApiEndpoint(settings: AppSettings): string {
  switch (settings.model.provider) {
    case 'ollama':
      return `${settings.model.baseUrl || 'http://localhost:11434'}/api/chat`;
    case 'openai':
      return settings.model.baseUrl || 'https://api.openai.com/v1/chat/completions';
    case 'anthropic':
      return settings.model.baseUrl || 'https://api.anthropic.com/v1/messages';
    default:
      return settings.model.baseUrl || '/api/chat';
  }
}

function getApiHeaders(settings: AppSettings): Record<string, string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (settings.model.apiKey) {
    headers['Authorization'] = `Bearer ${settings.model.apiKey}`;
  }

  // Provider-specific headers
  if (settings.model.provider === 'anthropic') {
    headers['anthropic-version'] = '2023-06-01';
  }

  return headers;
}
