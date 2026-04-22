import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import rustApiClient from '../config/api'

interface Position {
  poll_id: number
  poll_title: string
  option: string
  shares: number
  pnl?: number
  current_price?: number
  status: string
}

export default function Portfolio() {
  const [positions, setPositions] = useState<Position[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    rustApiClient.get<Position[]>('/api/v1/positions').then(res => setPositions(res.data)).finally(() => setLoading(false))
  }, [])

  const totalPnl = positions.reduce((s, p) => s + (p.pnl || 0), 0)

  if (loading) return <div className="flex justify-center py-20"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-purple-600" /></div>

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-2xl font-bold text-white mb-2">My Portfolio</h1>
      <p className={`text-lg font-semibold mb-6 ${totalPnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
        Total P&L: {totalPnl >= 0 ? '+' : ''}Kes {totalPnl.toFixed(2)}
      </p>

      {positions.length === 0 ? (
        <div className="glass-card-light rounded-xl p-12 text-center">
          <p className="text-gray-500 mb-4">No open positions yet.</p>
          <Link to="/markets" className="bg-gradient-polypulse-hero text-white px-5 py-2 rounded-lg hover:opacity-90 inline-block transition-all glow-purple">Browse Markets</Link>
        </div>
      ) : (
        <div className="space-y-3">
          {positions.map((pos, i) => (
            <Link key={i} to={`/markets/${pos.poll_id}`} className="block glass-card-light rounded-xl hover:glow-purple transition-all p-5">
              <div className="flex items-center justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <p className="font-semibold text-gray-900 truncate">{pos.poll_title}</p>
                  <p className="text-sm text-gray-600 mt-0.5">
                    <span className="font-medium text-purple-600">{pos.option}</span>
                    {' · '}{pos.shares} shares
                  </p>
                </div>
                <div className="text-right shrink-0">
                  <p className={`font-bold ${(pos.pnl ?? 0) >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {(pos.pnl ?? 0) >= 0 ? '+' : ''}Kes {pos.pnl?.toFixed(2)}
                  </p>
                  <p className="text-xs text-gray-500 mt-0.5">
                    {Math.round((pos.current_price || 0) * 100)}% · {pos.status}
                  </p>
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}
