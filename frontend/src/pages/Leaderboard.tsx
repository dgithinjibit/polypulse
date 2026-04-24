import React, { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import rustApiClient from '../config/api'
import { useAuth } from '../context/AuthContext'
import LegalPageNav from '../components/LegalPageNav'

interface LeaderboardEntry {
  rank: number
  username: string
  total_won: number
  accuracy: number
  current_streak: number
}

interface MyStats {
  rank?: number
  total_won: number
  accuracy: number
  current_streak: number
}

interface LeaderboardData {
  leaderboard: LeaderboardEntry[]
  my_stats: MyStats | null
}

export default function Leaderboard() {
  const { user } = useAuth()
  const [data, setData] = useState<LeaderboardData>({ leaderboard: [], my_stats: null })
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    // Show coming soon message for now
    setData({
      leaderboard: [],
      my_stats: null
    })
    setLoading(false)
  }, [])

  if (loading) return <div className="flex justify-center py-20"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" /></div>

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <LegalPageNav currentPage="leaderboard" />
      
      {/* Hero Header - Similar to Prediction Markets */}
      <div className="bg-gradient-polypulse-hero rounded-2xl p-8 mb-8 text-white shadow-xl">
        <h1 className="text-3xl font-bold mb-2">Leaderboard</h1>
        <p className="text-indigo-100 mb-4">Top traders ranked by performance. Compete for the highest accuracy and win streaks.</p>
        <Link
          to="/markets"
          className="inline-block bg-white text-indigo-700 font-semibold px-5 py-2 rounded-lg hover:bg-indigo-50 transition-all"
        >
          Browse Markets
        </Link>
      </div>

      {data.my_stats && (
        <div className="bg-purple-50 border border-purple-200 rounded-xl p-5 mb-6">
          <p className="text-sm font-medium text-purple-700 mb-2">Your Stats</p>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-center">
            {[
              { label: 'Rank', value: `#${data.my_stats.rank || '—'}` },
              { label: 'Total Won', value: `Kes ${data.my_stats.total_won}` },
              { label: 'Accuracy', value: `${data.my_stats.accuracy}%` },
              { label: 'Streak', value: `${data.my_stats.current_streak}` },
            ].map(s => (
              <div key={s.label} className="bg-white rounded-lg p-3">
                <p className="text-xs text-gray-500">{s.label}</p>
                <p className="font-bold text-gray-900">{s.value}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="bg-white rounded-xl shadow overflow-hidden">
        <table className="w-full">
          <thead className="bg-gray-50 border-b border-gray-200">
            <tr>
              {['Rank', 'Trader', 'Total Won', 'Accuracy', 'Streak'].map(h => (
                <th key={h} className="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">{h}</th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-100">
            {data.leaderboard.map(entry => (
              <tr key={entry.rank} className={`hover:bg-gray-50 ${user?.username === entry.username ? 'bg-purple-50' : ''}`}>
                <td className="px-4 py-3 text-sm font-bold text-gray-700">
                  {entry.rank <= 3 ? ['1st', '2nd', '3rd'][entry.rank - 1] : `#${entry.rank}`}
                </td>
                <td className="px-4 py-3 text-sm font-medium text-gray-900">{entry.username}</td>
                <td className="px-4 py-3 text-sm text-green-600 font-medium">Kes {entry.total_won}</td>
                <td className="px-4 py-3 text-sm text-gray-600">{entry.accuracy}%</td>
                <td className="px-4 py-3 text-sm text-gray-600">{entry.current_streak}</td>
              </tr>
            ))}
            {data.leaderboard.length === 0 && (
              <tr><td colSpan={5} className="px-4 py-8 text-center text-gray-400">No data yet.</td></tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
