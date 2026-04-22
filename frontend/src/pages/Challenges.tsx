import React, { useEffect, useState, FormEvent } from 'react'
import rustApiClient from '../config/api'
import { useAuth } from '../context/AuthContext'
import { useNavigate } from 'react-router-dom'
import { handleError, handleSuccess } from '../lib/error-handler'

interface Challenge {
  id: number
  question: string
  amount: number
  creator_username: string
  creator_choice: string
  status: 'pending' | 'accepted' | 'resolved' | 'cancelled' | 'expired'
  is_open: boolean
  winner_username?: string
}

interface ChallengeForm {
  question: string
  amount: string
  creator_choice: string
  expires_at: string
  is_open: boolean
}

const STATUS_COLOR: Record<string, string> = {
  pending: 'bg-yellow-100 text-yellow-700',
  accepted: 'bg-blue-100 text-blue-700',
  resolved: 'bg-green-100 text-green-700',
  cancelled: 'bg-gray-100 text-gray-600',
  expired: 'bg-red-100 text-red-600',
}

export default function Challenges() {
  const { user } = useAuth()
  const navigate = useNavigate()
  const [challenges, setChallenges] = useState<Challenge[]>([])
  const [loading, setLoading] = useState(true)
  const [showForm, setShowForm] = useState(false)
  const [form, setForm] = useState<ChallengeForm>({ question: '', amount: '', creator_choice: '', expires_at: '', is_open: true })
  const [formError, setFormError] = useState('')

  const load = () => {
    rustApiClient.get<Challenge[] | { results: Challenge[] }>('/api/v1/challenges').then(res => {
      const data = res.data
      setChallenges(Array.isArray(data) ? data : (data.results || []))
    }).finally(() => setLoading(false))
  }

  useEffect(() => { load() }, [])

  const createChallenge = async (e: FormEvent) => {
    e.preventDefault()
    setFormError('')
    try {
      await rustApiClient.post('/api/v1/challenges', { ...form, amount: parseFloat(form.amount) })
      setShowForm(false)
      setForm({ question: '', amount: '', creator_choice: '', expires_at: '', is_open: true })
      handleSuccess('Challenge Created', 'Your challenge has been created successfully.')
      load()
    } catch (err: unknown) {
      const axiosErr = err as { response?: { data?: Record<string, string[]> | string } }
      const data = axiosErr.response?.data
      const errorMsg = typeof data === 'object' && data ? Object.values(data).flat()[0] : 'Failed to create challenge'
      setFormError(errorMsg)
      handleError(err, {
        title: 'Challenge Creation Failed',
        onRetry: () => createChallenge(e),
      })
    }
  }

  const acceptChallenge = async (id: number, isOpen: boolean) => {
    if (!user) { navigate('/login'); return }
    try {
      await rustApiClient.post(`/api/v1/challenges/${id}/${isOpen ? 'accept-open' : 'accept'}`)
      handleSuccess('Challenge Accepted', 'You have successfully accepted the challenge.')
      load()
    } catch (err: unknown) {
      handleError(err, {
        title: 'Failed to Accept Challenge',
        onRetry: () => acceptChallenge(id, isOpen),
      })
    }
  }

  if (loading) return <div className="flex justify-center py-20"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" /></div>

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">Challenges</h1>
        {user && (
          <button onClick={() => setShowForm(!showForm)} className="bg-gradient-polypulse-hero text-white px-4 py-2 rounded-lg text-sm hover:opacity-90 shadow-lg">
            + New Challenge
          </button>
        )}
      </div>

      {showForm && (
        <div className="bg-white rounded-xl shadow p-6 mb-6">
          <h2 className="font-semibold text-gray-900 mb-4">Create Challenge</h2>
          {formError && <div className="bg-red-50 text-red-700 text-sm px-3 py-2 rounded mb-3">{formError}</div>}
          <form onSubmit={createChallenge} className="space-y-3">
            <input value={form.question} onChange={e => setForm({ ...form, question: e.target.value })} placeholder="Question / prediction" className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500" required />
            <div className="grid grid-cols-2 gap-3">
              <input type="number" value={form.amount} onChange={e => setForm({ ...form, amount: e.target.value })} placeholder="Amount (Kes)" className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500" required />
              <input value={form.creator_choice} onChange={e => setForm({ ...form, creator_choice: e.target.value })} placeholder="Your pick (e.g. Yes)" className="border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500" required />
            </div>
            <input type="datetime-local" value={form.expires_at} onChange={e => setForm({ ...form, expires_at: e.target.value })} className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500" required />
            <label className="flex items-center gap-2 text-sm text-gray-600">
              <input type="checkbox" checked={form.is_open} onChange={e => setForm({ ...form, is_open: e.target.checked })} />
              Open challenge (anyone can accept)
            </label>
            <div className="flex gap-2">
              <button type="submit" className="bg-gradient-polypulse-hero text-white px-4 py-2 rounded-lg text-sm hover:opacity-90 shadow-lg">Create</button>
              <button type="button" onClick={() => setShowForm(false)} className="border border-gray-300 px-4 py-2 rounded-lg text-sm hover:bg-gray-50">Cancel</button>
            </div>
          </form>
        </div>
      )}

      {challenges.length === 0 ? (
        <div className="bg-white rounded-xl shadow p-12 text-center text-gray-400">No challenges yet.</div>
      ) : (
        <div className="space-y-3">
          {challenges.map(c => (
            <div key={c.id} className="bg-white rounded-xl shadow p-5">
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1">
                  <p className="font-semibold text-gray-900">{c.question}</p>
                  <p className="text-sm text-gray-500 mt-1">
                    <span className="font-medium">{c.creator_username}</span> picks <span className="text-indigo-600 font-medium">&quot;{c.creator_choice}&quot;</span>
                    {' · '}Kes {c.amount}
                  </p>
                  {c.winner_username && <p className="text-sm text-green-600 mt-1">Winner: {c.winner_username}</p>}
                </div>
                <div className="flex flex-col items-end gap-2">
                  <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${STATUS_COLOR[c.status] || ''}`}>{c.status}</span>
                  {c.status === 'pending' && c.is_open && user && c.creator_username !== user.username && (
                    <button onClick={() => acceptChallenge(c.id, true)} className="text-xs bg-gradient-polypulse-hero text-white px-3 py-1 rounded hover:opacity-90 shadow">Accept</button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
