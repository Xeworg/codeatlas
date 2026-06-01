import { create } from 'zustand'
import type { ChatMessage } from '../lib/types'

interface ChatState {
  // State
  messages: ChatMessage[]
  isLoading: boolean
  error: string | null

  // Actions
  addMessage: (message: ChatMessage) => void
  clearMessages: () => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
}

export const useChatStore = create<ChatState>((set) => ({
  messages: [],
  isLoading: false,
  error: null,

  addMessage: (message) => set((state) => ({ messages: [...state.messages, message] })),

  clearMessages: () => set({ messages: [], error: null }),

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error, isLoading: false }),
}))

// Selectors
export const useChatMessages = () => useChatStore((s) => s.messages)
export const useIsChatLoading = () => useChatStore((s) => s.isLoading)
