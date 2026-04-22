import { useState, useCallback, useEffect, useRef } from 'react'
import rustApiClient, { CHAT_ENDPOINTS } from '../config/api'
import type { ChatMessage, SendMessagePayload } from '../types/wager'

interface UseChatReturn {
  messages: ChatMessage[]
  isLoading: boolean
  isSending: boolean
  error: string | null
  sendMessage: (content: string) => Promise<boolean>
  loadMessages: () => Promise<void>
}

export function useChat(wagerId: string): UseChatReturn {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [isSending, setIsSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const wsRef = useRef<WebSocket | null>(null)

  const loadMessages = useCallback(async () => {
    if (!wagerId) return
    setIsLoading(true)
    setError(null)
    try {
      const res = await rustApiClient.get<ChatMessage[]>(CHAT_ENDPOINTS.MESSAGES(wagerId))
      setMessages(res.data)
    } catch (err: unknown) {
      setError(getErrorMessage(err))
    } finally {
      setIsLoading(false)
    }
  }, [wagerId])

  const sendMessage = useCallback(async (content: string): Promise<boolean> => {
    if (!wagerId || !content.trim()) return false
    setIsSending(true)
    setError(null)
    try {
      const payload: SendMessagePayload = { wagerId, content: content.trim() }
      const res = await rustApiClient.post<ChatMessage>(CHAT_ENDPOINTS.SEND(wagerId), payload)
      setMessages((prev) => [...prev, res.data])
      return true
    } catch (err: unknown) {
      setError(getErrorMessage(err))
      return false
    } finally {
      setIsSending(false)
    }
  }, [wagerId])

  // WebSocket for real-time messages
  useEffect(() => {
    if (!wagerId) return

    const wsUrl = (import.meta.env.VITE_WS_URL || 'ws://localhost:8080') + `/ws/wagers/${wagerId}/chat`
    const ws = new WebSocket(wsUrl)
    wsRef.current = ws

    ws.onmessage = (event) => {
      try {
        const msg: ChatMessage = JSON.parse(event.data)
        setMessages((prev) => {
          // Avoid duplicates
          if (prev.some((m) => m.id === msg.id)) return prev
          return [...prev, msg]
        })
      } catch {
        // ignore malformed messages
      }
    }

    ws.onerror = () => {
      // WebSocket errors are non-fatal; REST polling is the fallback
    }

    return () => {
      ws.close()
      wsRef.current = null
    }
  }, [wagerId])

  return {
    messages,
    isLoading,
    isSending,
    error,
    sendMessage,
    loadMessages,
  }
}

function getErrorMessage(err: unknown): string {
  if (err && typeof err === 'object' && 'response' in err) {
    const axiosErr = err as { response?: { data?: { message?: string } } }
    return axiosErr.response?.data?.message || 'An error occurred'
  }
  if (err instanceof Error) return err.message
  return 'An error occurred'
}
