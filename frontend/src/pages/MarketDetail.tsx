import React, { useEffect, useState, useCallback } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts'
import rustApiClient from '../config/api'
import { useAuth } from '../context/AuthContext'

interface PollOption {
  id: number
  text: string
}

interface Poll {
  id: number
  title: string
  description: string
  status: string
  creator_username: string
  closes_at?: string
  category?: string
  options?: PollOption[]
  market?: { prices?: Record<number, number> }
}

interface PriceHistory {
  created_at: string
  yes_price: number
  no_price: number
}

interface Comment {
  id: number
  content: string
  user?: { username: string }
}

interface BetMsg {
  type: 'success' | 'error'
  text: string
}

function PriceBar({ option, price }: { option: PollOption; price: number }) {
  const pct = Math.round((price || 0) * 100)
  return (
    <div className="mb-3">
      <div className="flex justify-between text-sm mb-1">
        <span className="font-medium text-gray-800">{option.text}</span>
        <span className="font-bold text-purple-600">{pct}%</span>
      </div>
      <div className="w-full bg-purple-100/30 rounded-full h-2.5">
        <div className="bg-gradient-polypulse-hero h-2.5 rounded-full transition-all glow-purple" style={{ width: `${pct}%` }} />
      </div>
    </div>
  )
}

export default function MarketDetail() {
  const { id } = useParams<{ id: string }>()
  const { user, refreshUser } = useAuth()
  const navigate = useNavigate()
  const [poll, setPoll] = useState<Poll | null>(null)
  const [history, setHistory] = useState<PriceHistory[]>([])
  const [comments, setComments] = useState<Comment[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedOption, setSelectedOption] = useState<number | null>(null)
  const [amount, setAmount] = useState('')
  const [betLoading, setBetLoading] = useState(false)
  const [betMsg, setBetMsg] = useState<BetMsg | null>(null)
  const [comment, setComment] = useState('')

  const load = useCallback(() => {
    if (!id) return
    Promise.all([
      rustApiClient.get<Poll>(`/api/v1/polls/${id}`),
      rustApiClient.get<PriceHistory[]>(`/api/v1/polls/${id}/market-history`).catch(() => ({ data: [] })),
      rustApiClient.get<Comment[]>(`/api/v1/polls/${id}/comments`).catch(() => ({ data: [] })),
    ]).then(([p, h, c]) => {
      setPoll(p.data)
      setHistory(h.data)
      setComments(c.data)
      if (p.data.options?.length) setSelectedOption(p.data.options[0].id)
    }).finally(() => setLoading(false))
  }, [id])

  useEffect(() => { load() }, [load])

  useEffect(() => {
    if (!id) return
    const token = localStorage.getItem('access_token')
    if (!token) return

    const wsBase = import.meta.env.VITE_WS_URL || (
      window.location.protocol === 'https:' ? 'wss://' : 'ws://'
    ) + (import.meta.env.VITE_API_HOST || 'localhost:8000')

    const ws = new WebSocket(`${wsBase}/ws/market/${id}/?token=${encodeURIComponent(token)}`)
    ws.onmessage = (e) => {
      try {
        const data = JSON.parse(e.data) as { prices?: Record<number, number> }
        if (data.prices) {
          setPoll(prev => prev ? { ...prev, market: { ...prev.market, prices: data.prices } } : prev)
        }
      } catch {
        // ignore malformed messages
      }
    }
    return () => ws.close()
  }, [id])

  const placeBet = async () => {
    if (!user) { navigate('/login'); return }
    if (!selectedOption || !amount) return
    setBetLoading(true)
    setBetMsg(null)
    try {
      const res = await rustApiClient.post<{ shares: number; new_price: number }>(
        '/api/v1/bets',
        { poll: parseInt(id!), option: selectedOption, amount: parseFloat(amount) }
      )
      setBetMsg({ type: 'success', text: `Bought ${res.data.shares} shares at ${Math.round(res.data.new_price * 100)}%` })
      setAmount('')
      refreshUser()
      load()
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: { detail?: string; error?: string } } }
      setBetMsg({ type: 'error', text: axiosErr.response?.data?.detail || axiosErr.response?.data?.error || 'Trade failed' })
    } finally {
      setBetLoading(false)
    }
  }

  const postComment = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!user) { navigate('/login'); return }
    try {
      await rustApiClient.post(`/api/v1/polls/${id}/comments`, { content: comment })
      setComment('')
      load()
    } catch { /* ignore */ }
  }

  if (loading) return <div className="flex justify-center py-20"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" /></div>
  if (!poll) return <div className="text-center py-20 text-gray-400">Market not found.</div>

  const canTrade = poll.status === 'open'

  return (
    <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          <div className="glass-card-light rounded-xl p-6">
            <div className="flex items-start justify-between gap-4 mb-3">
              <h1 className="text-xl font-bold text-gray-900">{poll.title}</h1>
              <span className={`shrink-0 text-xs px-2 py-1 rounded-full font-medium ${poll.status === 'open' ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'}`}>
                {poll.status}
              </span>
            </div>
            <p className="text-gray-700 text-sm mb-4">{poll.description}</p>
            <div className="flex flex-wrap gap-4 text-xs text-gray-600">
              <span>Created by <strong>{poll.creator_username}</strong></span>
              <span>Closes {poll.closes_at ? new Date(poll.closes_at).toLocaleString() : '—'}</span>
              {poll.category && <span className="bg-purple-100 text-purple-700 px-2 py-0.5 rounded">{poll.category}</span>}
            </div>
          </div>

          <div className="glass-card-light rounded-xl p-6">
            <h2 className="font-semibold text-gray-900 mb-4">Current Prices</h2>
            {poll.options?.map(opt => (
              <PriceBar key={opt.id} option={opt} price={poll.market?.prices?.[opt.id] ?? (1 / (poll.options?.length || 1))} />
            ))}
          </div>

          {history.length > 1 && (
            <div className="glass-card-light rounded-xl p-6">
              <h2 className="font-semibold text-gray-900 mb-4">Price History</h2>
              <ResponsiveContainer width="100%" height={200}>
                <LineChart data={history}>
                  <XAxis dataKey="created_at" hide />
                  <YAxis domain={[0, 1]} tickFormatter={(v: number) => `${Math.round(v * 100)}%`} width={40} />
                  <Tooltip formatter={(v: number) => `${Math.round(v * 100)}%`} />
                  <Line type="monotone" dataKey="yes_price" stroke="#7c3aed" dot={false} name="Yes" />
                  <Line type="monotone" dataKey="no_price" stroke="#3b82f6" dot={false} name="No" />
                </LineChart>
              </ResponsiveContainer>
            </div>
          )}

          <div className="glass-card-light rounded-xl p-6">
            <h2 className="font-semibold text-gray-900 mb-4">Comments</h2>
            {user && (
              <form onSubmit={postComment} className="flex gap-2 mb-4">
                <input
                  value={comment}
                  onChange={e => setComment(e.target.value)}
                  placeholder="Add a comment..."
                  className="flex-1 glass-card-light border-0 rounded-lg px-3 py-2 text-sm text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-400"
                  required
                />
                <button type="submit" className="bg-gradient-polypulse-hero text-white px-4 py-2 rounded-lg text-sm hover:opacity-90 glow-purple">Post</button>
              </form>
            )}
            {comments.length === 0 ? (
              <p className="text-gray-500 text-sm">No comments yet.</p>
            ) : (
              <div className="space-y-3">
                {comments.map(c => (
                  <div key={c.id} className="flex gap-3">
                    <div className="w-8 h-8 rounded-full bg-purple-100 flex items-center justify-center text-purple-600 font-bold text-sm shrink-0">
                      {c.user?.username?.[0]?.toUpperCase() || '?'}
                    </div>
                    <div>
                      <p className="text-sm font-medium text-gray-800">{c.user?.username}</p>
                      <p className="text-sm text-gray-700">{c.content}</p>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        <div className="space-y-4">
          <div className="glass-card-light rounded-xl p-6 sticky top-4">
            <h2 className="font-semibold text-gray-900 mb-4">
              {canTrade ? 'Place Trade' : 'Trading Closed'}
            </h2>
            {canTrade ? (
              <>
                <div className="space-y-2 mb-4">
                  {poll.options?.map(opt => (
                    <button
                      key={opt.id}
                      onClick={() => setSelectedOption(opt.id)}
                      className={`w-full text-left px-3 py-2 rounded-lg border text-sm transition-all ${selectedOption === opt.id ? 'border-purple-500 bg-purple-100 text-purple-800 font-medium glow-purple' : 'border-purple-200 hover:border-purple-300 text-gray-700'}`}
                    >
                      {opt.text}
                    </button>
                  ))}
                </div>
                <div className="mb-4">
                  <label className="block text-sm font-medium text-gray-800 mb-1">Amount (Kes)</label>
                  <input
                    type="number"
                    min="1"
                    value={amount}
                    onChange={e => setAmount(e.target.value)}
                    placeholder="e.g. 100"
                    className="w-full glass-card-light border-0 rounded-lg px-3 py-2 text-sm text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-400"
                  />
                  {user && <p className="text-xs text-gray-600 mt-1">Balance: Kes {user.balance?.toFixed(2)}</p>}
                </div>
                {betMsg && (
                  <div className={`text-sm px-3 py-2 rounded mb-3 ${betMsg.type === 'success' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}`}>
                    {betMsg.text}
                  </div>
                )}
                <button
                  onClick={placeBet}
                  disabled={betLoading || !amount}
                  className="w-full bg-gradient-polypulse-hero hover:opacity-90 text-white font-medium py-2 rounded-lg transition-all disabled:opacity-50 glow-purple"
                >
                  {betLoading ? 'Processing...' : 'Buy Shares'}
                </button>
                {!user && <p className="text-xs text-center text-gray-500 mt-2">Login to trade</p>}
              </>
            ) : (
              <p className="text-gray-500 text-sm">This market is {poll.status}.</p>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
