import React from 'react'
import { useAuth } from '../context/AuthContext'
import { Link } from 'react-router-dom'

export default function Profile() {
  const { user } = useAuth()
  if (!user) return null

  const userWithExtras = user as typeof user & { is_admin?: boolean; polls_remaining_today?: number }

  return (
    <div className="max-w-2xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-2xl font-bold text-gray-900 mb-6">Profile</h1>
      <div className="bg-white rounded-xl shadow p-6 space-y-4">
        <div className="flex items-center gap-4">
          <div className="w-16 h-16 rounded-full bg-purple-100 flex items-center justify-center text-purple-600 font-bold text-2xl">
            {user.username?.[0]?.toUpperCase()}
          </div>
          <div>
            <p className="text-xl font-bold text-gray-900">{user.username}</p>
            <p className="text-gray-500 text-sm">{user.email}</p>
            {userWithExtras.is_admin && <span className="text-xs bg-purple-100 text-purple-700 px-2 py-0.5 rounded font-medium">Admin</span>}
          </div>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-3 gap-4 pt-4 border-t border-gray-100">
          {[
            { label: 'Balance', value: `Kes ${user.balance?.toFixed(2)}` },
            { label: 'Polls Remaining Today', value: userWithExtras.polls_remaining_today ?? '—' },
          ].map(s => (
            <div key={s.label} className="bg-gray-50 rounded-lg p-3">
              <p className="text-xs text-gray-500">{s.label}</p>
              <p className="font-bold text-gray-900 mt-0.5">{s.value}</p>
            </div>
          ))}
        </div>

        <div className="flex gap-3 pt-2">
          <Link to="/portfolio" className="bg-gradient-polypulse-hero text-white px-4 py-2 rounded-lg text-sm hover:opacity-90 shadow-lg">My Portfolio</Link>
          <Link to="/wallet" className="border border-gray-300 text-gray-700 px-4 py-2 rounded-lg text-sm hover:bg-gray-50">Wallet</Link>
        </div>
      </div>
    </div>
  )
}
