import React, { useEffect, useState } from 'react'
import rustApiClient from '../config/api'

interface Notification {
  id: number
  message: string
  is_read: boolean
  created_at: string
}

export default function Notifications() {
  const [notifications, setNotifications] = useState<Notification[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    rustApiClient.get<Notification[]>('/api/v1/notifications').then(res => setNotifications(res.data)).finally(() => setLoading(false))
  }, [])

  const markAll = async () => {
    await rustApiClient.post('/api/v1/notifications/read-all')
    setNotifications(n => n.map(x => ({ ...x, is_read: true })))
  }

  const markOne = async (id: number) => {
    await rustApiClient.post(`/api/v1/notifications/${id}/read`)
    setNotifications(n => n.map(x => x.id === id ? { ...x, is_read: true } : x))
  }

  if (loading) return <div className="flex justify-center py-20"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" /></div>

  return (
    <div className="max-w-2xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-white">Notifications</h1>
        {notifications.some(n => !n.is_read) && (
          <button onClick={markAll} className="text-sm text-purple-300 hover:text-purple-100 transition-colors">Mark all read</button>
        )}
      </div>

      {notifications.length === 0 ? (
        <div className="glass-card-light rounded-xl p-12 text-center text-gray-500">No notifications.</div>
      ) : (
        <div className="space-y-2">
          {notifications.map(n => (
            <div
              key={n.id}
              onClick={() => !n.is_read && markOne(n.id)}
              className={`glass-card-light rounded-xl p-4 cursor-pointer transition-all ${!n.is_read ? 'border-l-4 border-purple-500 glow-purple' : 'opacity-70'}`}
            >
              <p className="text-sm text-gray-800">{n.message}</p>
              <p className="text-xs text-gray-500 mt-1">{new Date(n.created_at).toLocaleString()}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
