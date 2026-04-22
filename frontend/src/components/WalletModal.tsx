import React from 'react'

export interface WalletOption {
  id: string
  name: string
  icon: string
  chain: 'stellar'
  detected: boolean
  installUrl: string
  onConnect: () => Promise<void>
}

interface Props {
  open: boolean
  onClose: () => void
  wallets: WalletOption[]
  connecting: string | null
}

export default function WalletModal({ open, onClose, wallets, connecting }: Props) {
  if (!open) return null

  const detected = wallets.filter(w => w.detected)
  const notInstalled = wallets.filter(w => !w.detected)

  return (
    <div 
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-modal="true"
      aria-labelledby="wallet-modal-title"
    >
      {/* Backdrop */}
      <div 
        className="absolute inset-0 bg-black/60 backdrop-blur-sm" 
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Modal */}
      <div className="relative bg-white rounded-2xl shadow-2xl w-full max-w-sm mx-4 overflow-hidden text-gray-900" role="document">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100">
          <h2 id="wallet-modal-title" className="text-lg font-bold text-gray-900">Connect Stellar Wallet</h2>
          <button 
            onClick={onClose} 
            className="text-gray-400 hover:text-gray-600 text-xl leading-none p-2 hover:bg-gray-100 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2"
            aria-label="Close wallet modal"
          >
            x
          </button>
        </div>

        <div className="px-6 py-4 space-y-3 max-h-[70vh] overflow-y-auto">
          {detected.length > 0 && (
            <>
              <p className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Detected</p>
              {detected.map(w => (
                <WalletRow key={w.id} wallet={w} connecting={connecting} />
              ))}
            </>
          )}

          {notInstalled.length > 0 && (
            <>
              <p className="text-xs font-semibold text-gray-400 uppercase tracking-wider mt-4">Not Installed</p>
              {notInstalled.map(w => (
                <WalletRow key={w.id} wallet={w} connecting={connecting} />
              ))}
            </>
          )}
        </div>

        <div className="px-6 py-3 bg-gray-50 border-t border-gray-100">
          <p className="text-xs text-gray-400 text-center">
            Your Stellar wallet is your identity — no account creation needed.
          </p>
        </div>
      </div>
    </div>
  )
}

function WalletRow({ wallet, connecting }: { wallet: WalletOption; connecting: string | null }) {
  const isConnecting = connecting === wallet.id

  if (!wallet.detected) {
    return (
      <a
        href={wallet.installUrl}
        target="_blank"
        rel="noopener noreferrer"
        className="flex items-center gap-3 p-4 rounded-xl border border-dashed border-gray-200 hover:border-gray-300 hover:bg-gray-50 transition-all group focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 min-h-[64px] touch-manipulation"
        aria-label={`Install ${wallet.name} wallet (opens in new tab)`}
      >
        <span className="text-2xl w-9 text-center flex-shrink-0" aria-hidden="true">{wallet.icon}</span>
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-gray-500">{wallet.name}</p>
          <p className="text-xs text-gray-400">Click to install</p>
        </div>
        <span className="text-xs px-2 py-0.5 rounded-full font-medium bg-yellow-50 text-yellow-700 flex-shrink-0" aria-label="Stellar network">
          STELLAR
        </span>
        <span className="text-gray-300 group-hover:text-gray-400 text-xs flex-shrink-0" aria-hidden="true">↗</span>
      </a>
    )
  }

  return (
    <button
      onClick={wallet.onConnect}
      disabled={!!connecting}
      className="w-full flex items-center gap-3 p-4 rounded-xl border border-gray-100 hover:border-purple-200 hover:bg-purple-50 transition-all disabled:opacity-60 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 min-h-[64px] touch-manipulation"
      aria-label={`Connect ${wallet.name} wallet`}
      aria-busy={isConnecting}
    >
      <span className="text-2xl w-9 text-center flex-shrink-0" aria-hidden="true">{wallet.icon}</span>
      <div className="flex-1 min-w-0 text-left">
        <p className="text-sm font-semibold text-gray-800">{wallet.name}</p>
        <p className="text-xs text-green-500 font-medium">● Ready to connect</p>
      </div>
      <span className="text-xs px-2 py-0.5 rounded-full font-medium bg-yellow-50 text-yellow-700 flex-shrink-0" aria-label="Stellar network">
        STELLAR
      </span>
      {isConnecting ? (
        <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-indigo-600 flex-shrink-0" role="status" aria-label="Connecting" />
      ) : (
        <span className="text-indigo-400 text-sm flex-shrink-0" aria-hidden="true">→</span>
      )}
    </button>
  )
}
