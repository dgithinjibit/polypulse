import React, { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import { useWagers } from '../hooks/useWagers'
import { useChat } from '../hooks/useChat'

export default function WagerRoom() {
  const { id } = useParams<{ id: string }>()
  const { currentWager, fetchWager, acceptWager, isLoading, error } = useWagers()
  const { messages, loadMessages, sendMessage, isSending } = useChat(id || '')
  const [messageInput, setMessageInput] = useState('')

  useEffect(() => {
    if (id) {
      fetchWager(id)
      loadMessages()
    }
  }, [id])

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!messageInput.trim()) return
    const ok = await sendMessage(messageInput)
    if (ok) setMessageInput('')
  }

  const handleAccept = async () => {
    if (!id) return
    await acceptWager(id)
    fetchWager(id)
  }

  if (isLoading && !currentWager) {
    return (
      <div className="flex justify-center py-20">
        <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" />
      </div>
    )
  }

  if (error) {
    return <div className="max-w-2xl mx-auto px-4 py-8 text-red-600">{error}</div>
  }

  if (!currentWager) {
    return <div className="max-w-2xl mx-auto px-4 py-8 text-gray-500">Wager not found.</div>
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8 grid grid-cols-1 lg:grid-cols-3 gap-6">
      {/* Wager details */}
      <div className="lg:col-span-2 space-y-4">
        <div className="bg-white rounded-xl shadow p-6">
          <div className="flex items-start justify-between gap-4 mb-4">
            <h1 className="text-xl font-bold text-gray-900">{currentWager.title}</h1>
            <span className={`text-xs px-2 py-1 rounded-full font-medium ${statusColor(currentWager.status)}`}>
              {currentWager.status}
            </span>
          </div>
          <p className="text-gray-600 text-sm mb-4">{currentWager.description}</p>
          <div className="border-t pt-4 space-y-2 text-sm text-gray-500">
            <p><span className="font-medium text-gray-700">Amount:</span> {currentWager.amount} {currentWager.currency}</p>
            <p><span className="font-medium text-gray-700">Resolution:</span> {currentWager.resolutionMethod.replace('_', ' ')}</p>
            <p><span className="font-medium text-gray-700">Expires:</span> {new Date(currentWager.expiresAt).toLocaleString()}</p>
            {currentWager.currentYield !== undefined && (
              <p><span className="font-medium text-green-600">Current Yield:</span> {currentWager.currentYield.toFixed(4)} {currentWager.currency}</p>
            )}
          </div>
          {currentWager.status === 'pending' && (
            <button
              onClick={handleAccept}
              disabled={isLoading}
              className="mt-4 w-full bg-gradient-polypulse-hero text-white font-semibold py-2 rounded-lg hover:opacity-90 disabled:opacity-50 transition-all shadow-lg"
            >
              Accept Wager
            </button>
          )}
        </div>

        {/* Participants */}
        <div className="bg-white rounded-xl shadow p-6">
          <h2 className="font-semibold text-gray-900 mb-3">Participants ({currentWager.participants.length}/{currentWager.maxParticipants})</h2>
          {currentWager.participants.length === 0 ? (
            <p className="text-sm text-gray-400">No participants yet.</p>
          ) : (
            <ul className="space-y-2">
              {currentWager.participants.map((p) => (
                <li key={p.id} className="text-sm text-gray-600 font-mono truncate">{p.displayName || p.address}</li>
              ))}
            </ul>
          )}
        </div>
      </div>

      {/* Chat */}
      <div className="bg-white rounded-xl shadow flex flex-col h-[500px]">
        <div className="px-4 py-3 border-b font-semibold text-gray-900 text-sm">Chat</div>
        <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3">
          {messages.length === 0 ? (
            <p className="text-xs text-gray-400 text-center mt-4">No messages yet.</p>
          ) : (
            messages.map((msg) => (
              <div key={msg.id} className="text-sm">
                <span className="font-medium text-indigo-600 mr-1">{msg.senderDisplayName || msg.senderAddress.slice(0, 8) + '...'}</span>
                <span className="text-gray-700">{msg.content}</span>
                <span className="text-xs text-gray-400 ml-2">{new Date(msg.sentAt).toLocaleTimeString()}</span>
              </div>
            ))
          )}
        </div>
        <form onSubmit={handleSend} className="px-4 py-3 border-t flex gap-2">
          <input
            value={messageInput}
            onChange={(e) => setMessageInput(e.target.value)}
            placeholder="Type a message..."
            maxLength={10000}
            className="flex-1 border border-gray-300 rounded-lg px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
          />
          <button
            type="submit"
            disabled={isSending || !messageInput.trim()}
            className="bg-gradient-polypulse-hero text-white px-3 py-1.5 rounded-lg text-sm hover:opacity-90 disabled:opacity-50 shadow-lg"
          >
            Send
          </button>
        </form>
      </div>
    </div>
  )
}

function statusColor(status: string): string {
  switch (status) {
    case 'active': return 'bg-green-100 text-green-700'
    case 'pending': return 'bg-yellow-100 text-yellow-700'
    case 'resolved': return 'bg-blue-100 text-blue-700'
    case 'cancelled':
    case 'expired': return 'bg-gray-100 text-gray-600'
    default: return 'bg-gray-100 text-gray-600'
  }
}
