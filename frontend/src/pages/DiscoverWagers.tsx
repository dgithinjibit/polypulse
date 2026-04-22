import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { useWagers } from '../hooks/useWagers'
import type { WagerListItem } from '../types/wager'
import type { DiscoverFilters } from '../hooks/useWagers'

export default function DiscoverWagers() {
  const { discoverWagers, isLoading, error } = useWagers()
  const [wagers, setWagers] = useState<WagerListItem[]>([])
  const [filters, setFilters] = useState<DiscoverFilters>({})

  useEffect(() => {
    discoverWagers(filters).then(setWagers)
  }, [])

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    discoverWagers(filters).then(setWagers)
  }

  return (
    <div className="max-w-5xl mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold text-gray-900 mb-6">Discover Wagers</h1>

      {/* Filters */}
      <form onSubmit={handleSearch} className="bg-white rounded-xl shadow p-4 mb-6 flex flex-wrap gap-3 items-end">
        <div>
          <label className="block text-xs font-medium text-gray-600 mb-1">Min Amount</label>
          <input
            type="number"
            min={0}
            placeholder="0"
            className="border border-gray-300 rounded-lg px-3 py-1.5 text-sm w-28 focus:outline-none focus:ring-2 focus:ring-indigo-500"
            onChange={(e) => setFilters((f) => ({ ...f, minAmount: Number(e.target.value) || undefined }))}
          />
        </div>
        <div>
          <label className="block text-xs font-medium text-gray-600 mb-1">Max Amount</label>
          <input
            type="number"
            min={0}
            placeholder="No limit"
            className="border border-gray-300 rounded-lg px-3 py-1.5 text-sm w-28 focus:outline-none focus:ring-2 focus:ring-indigo-500"
            onChange={(e) => setFilters((f) => ({ ...f, maxAmount: Number(e.target.value) || undefined }))}
          />
        </div>
        <button
          type="submit"
          className="bg-gradient-polypulse-hero text-white px-4 py-1.5 rounded-lg text-sm font-medium hover:opacity-90 shadow-lg"
        >
          Search
        </button>
      </form>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 rounded-lg p-3 mb-4 text-sm">{error}</div>
      )}

      {isLoading ? (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" />
        </div>
      ) : wagers.length === 0 ? (
        <div className="bg-white rounded-xl shadow p-12 text-center text-gray-400">No public wagers found.</div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {wagers.map((w) => (
            <WagerCard key={w.id} wager={w} />
          ))}
        </div>
      )}
    </div>
  )
}

function WagerCard({ wager }: { wager: WagerListItem }) {
  return (
    <Link
      to={`/wagers/${wager.id}`}
      className="block bg-white rounded-xl shadow hover:shadow-md transition-shadow p-5"
    >
      <div className="flex items-start justify-between gap-2 mb-2">
        <h3 className="font-semibold text-gray-900 text-sm line-clamp-2">{wager.title}</h3>
        <span className="text-xs px-2 py-0.5 rounded-full bg-purple-50 text-purple-700 shrink-0">
          {wager.amount} {wager.currency}
        </span>
      </div>
      <p className="text-xs text-gray-500 line-clamp-2 mb-3">{wager.description}</p>
      <div className="flex items-center justify-between text-xs text-gray-400">
        <span>{wager.participantCount}/{wager.maxParticipants} participants</span>
        <span>Expires {new Date(wager.expiresAt).toLocaleDateString()}</span>
      </div>
    </Link>
  )
}
