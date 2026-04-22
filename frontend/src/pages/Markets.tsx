import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import rustApiClient from '../config/api'

interface Category {
  slug: string
  name: string
}

interface Poll {
  id: number
  title: string
  description: string
  status: string
  creator_username: string
  closes_at?: string
  category?: string
}

const STATUS_TABS = ['all', 'open', 'closed', 'resolved'] as const

export default function Markets() {
  const [polls, setPolls] = useState<Poll[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const [loading, setLoading] = useState(true)
  const [status, setStatus] = useState('open')
  const [category, setCategory] = useState('')
  const [search, setSearch] = useState('')

  useEffect(() => {
    rustApiClient.get<Category[]>('/api/v1/categories').then(res => setCategories(res.data)).catch(() => {})
  }, [])

  useEffect(() => {
    setLoading(true)
    const params = new URLSearchParams()
    if (status !== 'all') params.set('status', status)
    if (category) params.set('category', category)
    rustApiClient.get<Poll[] | { results: Poll[] }>(`/api/v1/polls?${params}`).then(res => {
      const data = res.data
      setPolls(Array.isArray(data) ? data : (data.results || []))
    }).finally(() => setLoading(false))
  }, [status, category])

  const filtered = polls.filter(p =>
    !search || p.title.toLowerCase().includes(search.toLowerCase())
  )

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-2xl font-bold text-white mb-6">Prediction Markets</h1>

      <div className="flex flex-wrap gap-3 mb-6">
        <input
          type="text"
          placeholder="Search markets..."
          value={search}
          onChange={e => setSearch(e.target.value)}
          className="glass-card-light border-0 rounded-lg px-3 py-2 text-sm text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-400 w-64"
        />
        <select
          value={category}
          onChange={e => setCategory(e.target.value)}
          className="glass-card-light border-0 rounded-lg px-3 py-2 text-sm text-gray-900 focus:outline-none focus:ring-2 focus:ring-purple-400"
        >
          <option value="">All Categories</option>
          {categories.map(c => <option key={c.slug} value={c.slug}>{c.name}</option>)}
        </select>
        <div className="flex rounded-lg glass-card-light overflow-hidden">
          {STATUS_TABS.map(s => (
            <button
              key={s}
              onClick={() => setStatus(s)}
              className={`px-3 py-2 text-sm capitalize transition-all ${status === s ? 'bg-gradient-polypulse-hero text-white glow-purple' : 'text-gray-700 hover:bg-white/30'}`}
            >
              {s}
            </button>
          ))}
        </div>
      </div>

      {loading ? (
        <div className="flex justify-center py-16"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-purple-400" /></div>
      ) : filtered.length === 0 ? (
        <div className="glass-card-light rounded-xl p-12 text-center text-gray-400">No markets found.</div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          {filtered.map(poll => (
            <Link key={poll.id} to={`/markets/${poll.id}`} className="glass-card-light rounded-xl hover:glow-purple transition-all p-5 block">
              <div className="flex items-start justify-between gap-2 mb-3">
                <h3 className="font-semibold text-gray-900 line-clamp-2 flex-1">{poll.title}</h3>
                <span className={`shrink-0 text-xs px-2 py-0.5 rounded-full font-medium ${poll.status === 'open' ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'}`}>
                  {poll.status}
                </span>
              </div>
              <p className="text-sm text-gray-600 line-clamp-2 mb-3">{poll.description}</p>
              <div className="flex items-center justify-between text-xs text-gray-500">
                <span>by {poll.creator_username}</span>
                <span>{poll.closes_at ? new Date(poll.closes_at).toLocaleDateString() : '—'}</span>
              </div>
              {poll.category && (
                <span className="mt-2 inline-block text-xs bg-purple-100 text-purple-700 px-2 py-0.5 rounded">{poll.category}</span>
              )}
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
