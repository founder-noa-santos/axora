import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface ChatState {
  // UI state only - messages handled by assistant-ui runtime
  isSidebarOpen: boolean;
  activeConversationId: string | null;
  conversationIds: string[];
}

interface ChatActions {
  toggleSidebar: () => void;
  setActiveConversation: (id: string | null) => void;
  createConversation: () => string;
  deleteConversation: (id: string) => void;
}

export const useChatStore = create<ChatState & ChatActions>()(
  persist(
    (set, get) => ({
      isSidebarOpen: true,
      activeConversationId: null,
      conversationIds: [],

      toggleSidebar: () => {
        set((state) => ({ isSidebarOpen: !state.isSidebarOpen }));
      },

      setActiveConversation: (id) => {
        set({ activeConversationId: id });
      },

      createConversation: () => {
        const id = crypto.randomUUID();
        set((state) => ({
          conversationIds: [id, ...state.conversationIds],
          activeConversationId: id,
        }));
        return id;
      },

      deleteConversation: (id) => {
        set((state) => ({
          conversationIds: state.conversationIds.filter((cid) => cid !== id),
          activeConversationId:
            state.activeConversationId === id
              ? state.conversationIds[0] || null
              : state.activeConversationId,
        }));
      },
    }),
    {
      name: 'axora-chat',
    }
  )
);
