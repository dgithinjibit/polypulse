import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import rustApiClient from '../config/api'
import { useStellarWallet } from '../context/StellarWalletContext'

interface Stats {
  total_polls?: number
  new_polls_today?: number
  active_traders?: number
}

interface Poll {
  id: number
  title: string
  description: string
  status: string
  creator_username: string
  closes_at?: string
  options?: { id: number }[]
}

function StatCard({ label, value, color }: { label: string; value: string | number; color: string }) {
  return (
    <div className="glass-card-light rounded-xl p-6 hover:glow-purple transition-all">
      <p className="text-sm text-gray-600">{label}</p>
      <p className={`text-3xl font-bold mt-1 ${color}`}>{value}</p>
    </div>
  )
}

function PollCard({ poll }: { poll: Poll }) {
  const statusColor = poll.status === 'open' ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'
  return (
    <Link to={`/markets/${poll.id}`} className="block glass-card-light rounded-xl hover:glow-purple transition-all p-5">
      <div className="flex items-start justify-between gap-4">
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-gray-900 truncate">{poll.title}</h3>
          <p className="text-sm text-gray-600 mt-1 line-clamp-2">{poll.description}</p>
          <div className="flex items-center gap-2 mt-3">
            <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${statusColor}`}>{poll.status}</span>
            <span className="text-xs text-gray-500">by {poll.creator_username}</span>
          </div>
        </div>
        <div className="text-right shrink-0">
          <p className="text-sm font-medium text-gray-800">{poll.options?.length || 0} options</p>
          <p className="text-xs text-gray-500 mt-1">
            {poll.closes_at ? new Date(poll.closes_at).toLocaleDateString() : '—'}
          </p>
        </div>
      </div>
    </Link>
  )
}

export default function Home() {
  const { isConnected, connectWallet } = useStellarWallet()
  const [stats, setStats] = useState<Stats>({})
  const [polls, setPolls] = useState<Poll[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    // Fetch polls and calculate stats from the data
    rustApiClient.get<Poll[] | { results: Poll[] }>('/api/v1/polls?status=open')
      .then(p => {
        const pollData = p.data
        const list = Array.isArray(pollData) ? pollData : (pollData.results || [])
        setPolls(list.slice(0, 6))
        // Calculate stats from polls
        setStats({
          total_polls: list.length,
          new_polls_today: list.filter(poll => {
            const created = new Date(poll.closes_at || '')
            const today = new Date()
            return created.toDateString() === today.toDateString()
          }).length,
          active_traders: 0 // TODO: Get from backend when available
        })
      })
      .finally(() => setLoading(false))
  }, [])

  const handleConnect = async () => {
    try {
      await connectWallet()
    } catch (err) {
      console.error('Connection failed:', err)
    }
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="bg-gradient-polypulse-hero rounded-2xl p-8 mb-8 text-white glow-purple">
        <h1 className="text-3xl font-bold mb-2">Prediction Markets</h1>
        <p className="text-purple-100 mb-4">Trade on real-world outcomes. Buy and sell shares in predictions.</p>
        {!isConnected ? (
          <div className="flex gap-3">
            <button
              onClick={handleConnect}
              className="bg-white text-purple-700 font-semibold px-5 py-2 rounded-lg hover:bg-purple-50 transition-all"
            >
              Connect Wallet
            </button>
            <Link to="/markets" className="border-2 border-white text-white px-5 py-2 rounded-lg hover:bg-white/10 transition-all">Browse Markets</Link>
          </div>
        ) : (
          <Link to="/markets" className="bg-white text-purple-700 font-semibold px-5 py-2 rounded-lg hover:bg-purple-50 inline-block transition-all">Browse Markets</Link>
        )}
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-8">
        <StatCard label="Total Markets" value={stats.total_polls ?? '—'} color="text-purple-600" />
        <StatCard label="New Today" value={stats.new_polls_today ?? '—'} color="text-blue-600" />
        <StatCard label="Active Traders" value={stats.active_traders ?? '—'} color="text-indigo-600" />
      </div>

      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-white">Open Markets</h2>
        <Link to="/markets" className="text-purple-300 hover:text-purple-100 text-sm font-medium transition-colors">View all →</Link>
      </div>

      {loading ? (
        <div className="flex justify-center py-12"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-purple-400" /></div>
      ) : polls.length === 0 ? (
        <div className="glass-card-light rounded-xl p-12 text-center text-gray-500">No open markets yet.</div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {polls.map(p => <PollCard key={p.id} poll={p} />)}
        </div>
      )}
    </div>
  )
}
