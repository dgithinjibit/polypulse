import { useNavigate } from 'react-router-dom'
import { useStellarWallet } from '../context/StellarWalletContext'
import LoadingOverlay from '../components/LoadingOverlay'

export default function SocialLogin() {
  const { isConnected, isLoading, isAuthenticating, publicKey, connectWallet } = useStellarWallet()
  const navigate = useNavigate()

  const handleConnect = async () => {
    try {
      await connectWallet()
    } catch (err) {
      console.error('Connect failed:', err)
    }
  }

  const handleContinue = () => {
    navigate('/markets', { replace: true })
  }

  if (isConnected && publicKey) {
    return (
      <div className="flex items-center justify-center px-4 py-16">
        <LoadingOverlay isVisible={isAuthenticating} message="Authenticating with backend..." />
        <div className="max-w-md w-full bg-gray-900/80 border border-gray-700/50 rounded-2xl shadow-xl p-8 backdrop-blur-sm text-center">
          <div className="w-16 h-16 bg-gradient-polypulse-hero rounded-full flex items-center justify-center mx-auto mb-4 shadow-lg">
            <span className="text-white font-bold text-2xl">P</span>
          </div>
          <h1 className="text-2xl font-bold text-white mb-2">Welcome to PolyPulse!</h1>
          <p className="text-gray-400 text-sm mb-4">Your Stellar wallet is connected.</p>
          <div className="bg-gray-800/60 border border-gray-700/50 rounded-xl px-4 py-3 mb-6 font-mono text-sm text-purple-300 break-all">
            {publicKey}
          </div>

          <div className="space-y-3 mb-6 text-left">
            <div className="p-3 bg-gray-800/50 border border-gray-700/40 rounded-xl">
              <div className="font-semibold text-gray-100 text-sm">Predict Markets</div>
              <div className="text-xs text-gray-500">Trade on real-world outcomes using XLM</div>
            </div>
            <div className="p-3 bg-gray-800/50 border border-gray-700/40 rounded-xl">
              <div className="font-semibold text-gray-100 text-sm">Compete & Earn</div>
              <div className="text-xs text-gray-500">Climb the leaderboard and win rewards</div>
            </div>
            <div className="p-3 bg-gray-800/50 border border-gray-700/40 rounded-xl">
              <div className="font-semibold text-gray-100 text-sm">Fast & Low-Cost</div>
              <div className="text-xs text-gray-500">Powered by the Stellar network</div>
            </div>
          </div>

          <button
            onClick={handleContinue}
            className="w-full bg-gradient-polypulse-hero hover:opacity-90 text-white font-semibold py-3 px-4 rounded-xl transition-all shadow-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 min-h-[48px]"
          >
            Explore Markets
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="flex items-center justify-center px-4 py-16">
      <LoadingOverlay isVisible={isAuthenticating} message="Authenticating with backend..." />
      <div className="max-w-sm w-full bg-gray-900/80 border border-gray-700/50 rounded-2xl shadow-xl p-8 backdrop-blur-sm text-center">
        <div className="w-12 h-12 bg-gradient-polypulse-hero rounded-xl flex items-center justify-center mx-auto mb-4 shadow-lg">
          <span className="text-white font-bold text-xl">P</span>
        </div>
        <h1 className="text-2xl font-bold text-white mb-2">Connect Your Wallet</h1>
        <p className="text-gray-400 text-sm mb-8">
          Use your Stellar wallet to get started. No account needed.
        </p>

        <button
          onClick={handleConnect}
          disabled={isLoading || isAuthenticating}
          className="w-full flex items-center justify-center gap-3 bg-gradient-polypulse-hero text-white rounded-xl px-4 py-3 text-sm font-semibold hover:opacity-90 disabled:opacity-50 transition-all shadow-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 min-h-[48px]"
          aria-busy={isLoading}
        >
          {isLoading ? (
            <>
              <span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
              <span>Connecting...</span>
            </>
          ) : (
            'Connect Wallet'
          )}
        </button>

        <p className="text-xs text-gray-600 mt-6">
          Supports Freighter and other Stellar wallets.
        </p>
      </div>
    </div>
  )
}
