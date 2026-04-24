import React, { useState, useEffect } from 'react'
import { Link, useNavigate, useLocation } from 'react-router-dom'
import { useStellarWallet } from '../context/StellarWalletContext'
import { useWebSocket } from '../context/WebSocketContext'
import BalanceSkeleton from './BalanceSkeleton'
import NotificationBell from './NotificationBell'
import rustApiClient from '../config/api'

interface Notification {
  id: string
  is_read: boolean
}

function formatAddress(key: string): string {
  if (!key || key.length < 8) return key
  return `${key.slice(0, 4)}...${key.slice(-4)}`
}

export default function Navbar() {
  const { publicKey, isConnected, isLoading, isLoadingBalance, balance, connectWallet, disconnect } = useStellarWallet()
  const { connectionState, isConnected: wsConnected } = useWebSocket()

  const navigate = useNavigate()
  const location = useLocation()
  const [unread, setUnread] = useState(0)
  const [menuOpen, setMenuOpen] = useState(false)

  useEffect(() => {
    if (!isConnected) return
    rustApiClient.get<Notification[]>('/api/v1/notifications')
      .then(res => setUnread(res.data.filter(n => !n.is_read).length))
      .catch(() => {})
  }, [isConnected, location.pathname])

  const handleConnect = async () => {
    try {
      await connectWallet()
    } catch (err) {
      console.error('Connect failed:', err)
    }
  }

  const handleDisconnect = async () => {
    await disconnect()
    localStorage.removeItem('user_id')
    localStorage.removeItem('user_email')
    navigate('/')
  }

  const navLink = (to: string, label: string) => (
    <Link
      to={to}
      className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
        location.pathname === to
          ? 'bg-white/20 text-white backdrop-blur-sm'
          : 'text-purple-100 hover:bg-white/10 hover:text-white'
      }`}
    >
      {label}
    </Link>
  )

  // Get connection status indicator color and text
  // Requirement 8.7: Display connection status indicator in header
  const getConnectionStatus = () => {
    switch (connectionState) {
      case 'connected':
        return { color: 'bg-green-500', text: 'Live', title: 'Real-time updates active' };
      case 'connecting':
        return { color: 'bg-yellow-500', text: 'Connecting', title: 'Connecting to real-time updates' };
      case 'reconnecting':
        return { color: 'bg-yellow-500', text: 'Reconnecting', title: 'Reconnecting to real-time updates' };
      case 'disconnected':
        return { color: 'bg-gray-400', text: 'Offline', title: 'Real-time updates unavailable' };
      default:
        return { color: 'bg-gray-400', text: 'Offline', title: 'Real-time updates unavailable' };
    }
  };

  const connectionStatus = getConnectionStatus();

  return (
    <nav className="bg-gradient-polypulse shadow-lg">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          <div className="flex items-center gap-6">
            <Link to="/" className="text-white font-bold text-xl tracking-tight">
              PolyPulse
            </Link>
            <div className="hidden md:flex items-center gap-1">
              {navLink('/markets', 'Markets')}
              {navLink('/leaderboard', 'Leaderboard')}
              {navLink('/challenges', 'Challenges')}
              {isConnected && navLink('/portfolio', 'Portfolio')}
              {isConnected && navLink('/wallet', 'Wallet')}
            </div>
          </div>

          <div className="hidden md:flex items-center gap-3">
            {/* WebSocket Connection Status Indicator */}
            {isConnected && (
              <div 
                className="flex items-center gap-2 px-3 py-1.5 bg-white/10 rounded-lg backdrop-blur-sm"
                title={connectionStatus.title}
              >
                <div className={`w-2 h-2 ${connectionStatus.color} rounded-full ${wsConnected ? 'animate-pulse' : ''}`}></div>
                <span className="text-xs text-purple-100 font-medium">{connectionStatus.text}</span>
              </div>
            )}
            
            {isConnected ? (
              <>
                {isLoadingBalance ? (
                  <BalanceSkeleton />
                ) : balance?.xlm ? (
                  <span className="text-purple-100 text-sm">
                    <span className="font-bold text-white">{parseFloat(balance.xlm).toFixed(2)}</span> XLM
                  </span>
                ) : null}
                <NotificationBell />
                <Link 
                  to="/profile" 
                  className="text-purple-100 hover:text-white text-sm font-medium font-mono flex items-center gap-1 transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-700 rounded px-2 py-1"
                  aria-label={`Profile: ${publicKey ? formatAddress(publicKey) : 'View profile'}`}
                >
                  {publicKey ? formatAddress(publicKey) : ''}
                </Link>
                <button 
                  onClick={handleDisconnect} 
                  className="bg-white/20 hover:bg-white/30 text-white px-3 py-1.5 rounded text-sm transition-all backdrop-blur-sm focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-700"
                  aria-label="Disconnect wallet"
                >
                  Disconnect
                </button>
              </>
            ) : (
              <button
                onClick={handleConnect}
                disabled={isLoading}
                className="bg-white text-purple-700 hover:bg-purple-50 px-4 py-1.5 rounded text-sm font-medium disabled:opacity-50 flex items-center gap-2 transition-all shadow-lg focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-700"
                aria-label="Connect Stellar wallet"
                aria-busy={isLoading}
              >
                {isLoading ? (
                  <><span className="animate-spin rounded-full h-3 w-3 border-b-2 border-purple-700" aria-hidden="true" role="status" /><span>Connecting...</span></>
                ) : (
                  'Connect Wallet'
                )}
              </button>
            )}
          </div>

          {/* Mobile menu button */}
          <button 
            className="md:hidden text-purple-100 hover:text-white transition-colors p-2 -mr-2 focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-700 rounded" 
            onClick={() => setMenuOpen(!menuOpen)}
            aria-label={menuOpen ? "Close menu" : "Open menu"}
            aria-expanded={menuOpen}
            aria-controls="mobile-menu"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={menuOpen ? 'M6 18L18 6M6 6l12 12' : 'M4 6h16M4 12h16M4 18h16'} />
            </svg>
          </button>
        </div>
      </div>

      {/* Mobile menu */}
      {menuOpen && (
        <div 
          id="mobile-menu" 
          className="md:hidden bg-purple-900/50 backdrop-blur-sm px-4 pb-4 space-y-1"
          role="navigation"
          aria-label="Mobile navigation"
        >
          <Link to="/markets" className="block text-purple-100 py-2 hover:text-white transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-900 rounded px-2" onClick={() => setMenuOpen(false)}>Markets</Link>
          <Link to="/leaderboard" className="block text-purple-100 py-2 hover:text-white transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-900 rounded px-2" onClick={() => setMenuOpen(false)}>Leaderboard</Link>
          <Link to="/challenges" className="block text-purple-100 py-2 hover:text-white transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-900 rounded px-2" onClick={() => setMenuOpen(false)}>Challenges</Link>
          {isConnected && <Link to="/portfolio" className="block text-purple-100 py-2 hover:text-white transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-900 rounded px-2" onClick={() => setMenuOpen(false)}>Portfolio</Link>}
          {isConnected && <Link to="/wallet" className="block text-purple-100 py-2 hover:text-white transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-900 rounded px-2" onClick={() => setMenuOpen(false)}>Wallet</Link>}
          {isConnected && (
            <div className="py-2 border-t border-purple-700/50 mt-2">
              {isLoadingBalance ? (
                <BalanceSkeleton />
              ) : balance?.xlm ? (
                <span className="text-purple-100 text-sm block mb-2">
                  Balance: <span className="font-bold text-white">{parseFloat(balance.xlm).toFixed(2)}</span> XLM
                </span>
              ) : null}
              {publicKey && (
                <span className="text-purple-100 text-sm font-mono block mb-2">
                  {formatAddress(publicKey)}
                </span>
              )}
            </div>
          )}
          {isConnected ? (
            <button 
              onClick={handleDisconnect} 
              className="block w-full text-left text-red-300 py-2 hover:text-red-200 transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-900 rounded px-2"
              aria-label="Disconnect wallet"
            >
              Disconnect
            </button>
          ) : (
            <button 
              onClick={() => { setMenuOpen(false); handleConnect() }} 
              className="block w-full text-left text-purple-100 py-2 hover:text-white transition-colors focus:outline-none focus:ring-2 focus:ring-white focus:ring-offset-2 focus:ring-offset-purple-900 rounded px-2"
              aria-label="Connect Stellar wallet"
            >
              Connect Wallet
            </button>
          )}
        </div>
      )}
    </nav>
  )
}
