import React, { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useWagers } from '../hooks/useWagers'
import type { CreateWagerPayload, ResolutionMethod, DefiProtocol } from '../types/wager'

const defaultPayload: CreateWagerPayload = {
  title: '',
  description: '',
  resolutionCriteria: '',
  amount: 0,
  currency: 'USDC',
  maxParticipants: 2,
  expiresAt: '',
  resolutionMethod: 'ai_oracle',
  defiProtocol: 'aave',
  isPublic: false,
}

export default function CreateWager() {
  const navigate = useNavigate()
  const { createWager, isLoading, error } = useWagers()
  const [form, setForm] = useState<CreateWagerPayload>(defaultPayload)

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
    const { name, value, type } = e.target
    setForm((prev) => ({
      ...prev,
      [name]: type === 'number' ? Number(value) : value,
    }))
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const wager = await createWager(form)
    if (wager) {
      navigate(`/wagers/${wager.id}`)
    }
  }

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold text-gray-900 mb-6">Create a Wager</h1>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 rounded-lg p-3 mb-4 text-sm">{error}</div>
      )}

      <form onSubmit={handleSubmit} className="space-y-5 bg-white rounded-xl shadow p-6">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Title</label>
          <input
            name="title"
            value={form.title}
            onChange={handleChange}
            required
            className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
            placeholder="e.g. Will it rain in NYC on Friday?"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Description</label>
          <textarea
            name="description"
            value={form.description}
            onChange={handleChange}
            rows={3}
            className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
            placeholder="Describe the wager..."
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Resolution Criteria</label>
          <textarea
            name="resolutionCriteria"
            value={form.resolutionCriteria}
            onChange={handleChange}
            required
            rows={3}
            minLength={10}
            maxLength={2000}
            className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
            placeholder="How will this wager be resolved? (10–2000 characters)"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Amount (USDC)</label>
            <input
              name="amount"
              type="number"
              min={1}
              value={form.amount}
              onChange={handleChange}
              required
              className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Max Participants</label>
            <input
              name="maxParticipants"
              type="number"
              min={2}
              max={10}
              value={form.maxParticipants}
              onChange={handleChange}
              className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
            />
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Expires At</label>
          <input
            name="expiresAt"
            type="datetime-local"
            value={form.expiresAt}
            onChange={handleChange}
            required
            className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Resolution Method</label>
            <select
              name="resolutionMethod"
              value={form.resolutionMethod}
              onChange={handleChange}
              className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
            >
              <option value="ai_oracle">AI Oracle</option>
              <option value="trusted_judge">Trusted Judge</option>
              <option value="social_consensus">Social Consensus</option>
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">DeFi Protocol</label>
            <select
              name="defiProtocol"
              value={form.defiProtocol}
              onChange={handleChange}
              className="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
            >
              <option value="aave">Aave</option>
              <option value="movement">Movement</option>
            </select>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <input
            id="isPublic"
            name="isPublic"
            type="checkbox"
            checked={!!form.isPublic}
            onChange={(e) => setForm((prev) => ({ ...prev, isPublic: e.target.checked }))}
            className="h-4 w-4 text-indigo-600 border-gray-300 rounded"
          />
          <label htmlFor="isPublic" className="text-sm text-gray-700">Make this wager publicly discoverable</label>
        </div>

        <button
          type="submit"
          disabled={isLoading}
          className="w-full bg-gradient-polypulse-hero text-white font-semibold py-2.5 rounded-lg hover:opacity-90 disabled:opacity-50 transition-all shadow-lg"
        >
          {isLoading ? 'Creating...' : 'Create Wager'}
        </button>
      </form>
    </div>
  )
}
