import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useStellarWallet } from '../context/StellarWalletContext'

export default function Login() {
  const navigate = useNavigate()
  const { isConnected, isLoading, isAuthenticating, connectWallet } = useStellarWallet()

  useEffect(() => {
    if (isConnected) navigate('/social-login', { replace: true })
  }, [isConnected, navigate])

  const handleConnect = async () => {
    try {
      await connectWallet()
    } catch {
      // errors handled inside connectWallet via toast
    }
  }

  const busy = isLoading || isAuthenticating

  return (
    <div className="flex items-center justify-center px-4 py-16">
      <div className="max-w-md w-full bg-gray-900/80 border border-gray-700/50 rounded-2xl shadow-xl p-8 backdrop-blur-sm">
        <div className="text-center mb-8">
          <div className="w-14 h-14 bg-gradient-polypulse-hero rounded-2xl flex items-center justify-center mx-auto mb-4 shadow-lg">
            <span className="text-white font-bold text-2xl">P</span>
          </div>
          <h1 className="text-3xl font-bold text-white mb-2">Welcome to PolyPulse</h1>
          <p className="text-gray-400 text-sm">
            Connect your Stellar wallet to get started.
            <br />No account creation needed.
          </p>
        </div>

        <button
          onClick={handleConnect}
          disabled={busy}
          className="w-full flex items-center justify-center gap-3 bg-gradient-polypulse-hero hover:opacity-90 text-white font-semibold py-3.5 px-4 rounded-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-lg focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 min-h-[48px]"
        >
          {isAuthenticating ? (
            <><span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" /><span>Authenticating...</span></>
          ) : isLoading ? (
            <><span className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" /><span>Connecting...</span></>
          ) : (
            'Connect Wallet'
          )}
        </button>

        <div className="mt-6 border border-gray-700/50 rounded-xl p-4">
          <div className="mb-2">
            <div className="font-semibold text-gray-200 text-sm">Freighter</div>
            <div className="text-xs text-gray-500">Browser extension for Stellar</div>
          </div>
          <p className="text-xs text-gray-500">
            Don't have Freighter?{' '}
            <a href="https://www.freighter.app/" target="_blank" rel="noopener noreferrer" className="text-purple-400 hover:text-purple-300 hover:underline">
              Install it here
            </a>
          </p>
        </div>

        <p className="text-xs text-gray-600 text-center mt-6">
          By connecting, you agree to our Terms of Service and Privacy Policy.
        </p>
      </div>
    </div>
  )
}
