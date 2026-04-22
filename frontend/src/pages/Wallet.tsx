import React, { useEffect, useState } from 'react'
import rustApiClient from '../config/api'

interface Transaction {
  id: number
  transaction_type: string
  amount: number
  balance_after: number
  description?: string
  created_at: string
}

interface TransactionHistoryResponse {
  transactions: Transaction[]
  total: number
  limit: number
  offset: number
}

const TYPE_COLORS: Record<string, string> = {
  deposit: 'text-green-600',
  win: 'text-green-600',
  bet: 'text-red-600',
  refund: 'text-blue-600',
  admin_adjustment: 'text-purple-600',
}

export default function Wallet() {
  const [txns, setTxns] = useState<Transaction[]>([])
  const [loading, setLoading] = useState(true)

  const loadTxns = () => {
    rustApiClient.get<TransactionHistoryResponse>('/api/v1/wallet/transactions')
      .then(res => {
        // Extract transactions array from the response object
        const txnArray = res.data.transactions || []
        setTxns(txnArray)
      })
      .catch(err => {
        console.error('Failed to load transactions:', err)
        setTxns([]) // Set empty array on error to prevent crash
      })
      .finally(() => setLoading(false))
  }

  useEffect(() => { loadTxns() }, [])

  if (loading) return <div className="flex justify-center py-20"><div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" /></div>

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-white">Wallet</h1>
      </div>

      {txns.length === 0 ? (
        <div className="glass-card-light rounded-xl p-12 text-center text-gray-500">No transactions yet.</div>
      ) : (
        <div className="glass-card-light rounded-xl overflow-hidden">
          <table className="w-full">
            <thead className="bg-white/30 border-b border-purple-100">
              <tr>
                {['Type', 'Amount', 'Balance After', 'Description', 'Date'].map(h => (
                  <th key={h} className="px-4 py-3 text-left text-xs font-medium text-gray-700 uppercase">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-purple-100">
              {Array.isArray(txns) && txns.map(t => (
                <tr key={t.id} className="hover:bg-white/20 transition-colors">
                  <td className="px-4 py-3">
                    <span className={`text-xs font-medium capitalize ${TYPE_COLORS[t.transaction_type] || 'text-gray-600'}`}>
                      {t.transaction_type.replace('_', ' ')}
                    </span>
                  </td>
                  <td className={`px-4 py-3 text-sm font-bold ${t.amount >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {t.amount >= 0 ? '+' : ''}Kes {t.amount?.toFixed(2)}
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-700">Kes {t.balance_after?.toFixed(2)}</td>
                  <td className="px-4 py-3 text-sm text-gray-600 max-w-xs truncate">{t.description || '—'}</td>
                  <td className="px-4 py-3 text-xs text-gray-500">{new Date(t.created_at).toLocaleDateString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
