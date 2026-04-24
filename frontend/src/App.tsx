/**
 * ============================================================
 * FILE: App.tsx
 * PURPOSE: The root component of the PolyPulse frontend application.
 *          Sets up:
 *            1. Client-side routing (BrowserRouter + Routes)
 *            2. Global context providers (Auth, StellarWallet)
 *            3. Persistent layout (Navbar + Footer on every page)
 *            4. All application routes mapped to page components
 *            5. Toast notification system
 *
 * ARCHITECTURE NOTE:
 *   Providers are nested: BrowserRouter > AuthProvider > StellarWalletProvider
 *   This means StellarWalletProvider can use useNavigate() (from BrowserRouter)
 *   and AuthProvider is available to all components.
 *
 * ROUTE TYPES:
 *   - Public routes: accessible without wallet connection (/, /login, /markets, etc.)
 *   - Protected routes: wrapped in <ProtectedRoute>, redirect to /login if not connected
 * ============================================================
 */

// BrowserRouter: enables client-side routing using the browser's History API
// Routes: container for all Route definitions
// Route: maps a URL path to a component
import { BrowserRouter, Routes, Route } from 'react-router-dom'

// AuthProvider: provides user authentication state (JWT tokens, user profile)
import { AuthProvider } from './context/AuthContext'

// StellarWalletProvider: provides Stellar wallet state (public key, balance, connect/disconnect)
import { StellarWalletProvider } from './context/StellarWalletContext'

// WebSocketProvider: provides WebSocket connection state for real-time bet updates
import { WebSocketProvider } from './context/WebSocketContext'

// Navbar: persistent top navigation bar shown on every page
import Navbar from './components/Navbar'

// Footer: persistent bottom footer shown on every page
import { Footer } from './components/Footer'

// ProtectedRoute: wrapper that redirects to /login if wallet is not connected
import ProtectedRoute from './components/ProtectedRoute'

// Toaster: renders toast notification popups (success/error messages)
import { Toaster } from './components/ui/toaster'

// ---- Page Components ----
// Each of these is a full page rendered at its corresponding route

import Home from './pages/Home'                   // Landing page - /
import Login from './pages/Login'                 // Wallet connection page - /login
import Register from './pages/Register'           // Registration page - /register
import Markets from './pages/Markets'             // Browse prediction markets - /markets
import MarketDetail from './pages/MarketDetail'   // Single market detail - /markets/:id
import Portfolio from './pages/Portfolio'         // User's positions - /portfolio (protected)
import Leaderboard from './pages/Leaderboard'     // Top traders - /leaderboard
import Challenges from './pages/Challenges'       // Challenges list - /challenges
import Wallet from './pages/Wallet'               // Wallet management - /wallet (protected)
import Notifications from './pages/Notifications' // User notifications - /notifications (protected)
import Profile from './pages/Profile'             // User profile - /profile (protected)
import SocialLogin from './pages/SocialLogin'     // Post-connect onboarding - /social-login
import Help from './pages/Help'                   // Help/FAQ page - /help
import Terms from './pages/Terms'                 // Terms page - /terms
import Privacy from './pages/Privacy'             // Privacy page - /privacy
import BetDashboard from './pages/BetDashboard'   // P2P Betting Dashboard - /bets
import BetDetailPage from './pages/BetDetailPage' // P2P Bet Detail - /bet/:id

// ============================================================
// COMPONENT: App
// PURPOSE: Root component that assembles the entire application.
//          Rendered once by main.tsx and never unmounts.
// ============================================================
export default function App() {
  return (
    // BrowserRouter: must wrap everything that uses routing hooks (useNavigate, useLocation, etc.)
    <BrowserRouter>
      {/* AuthProvider: makes user auth state available via useAuth() hook */}
      <AuthProvider>
        {/* WebSocketProvider: makes WebSocket connection state available via useWebSocket() hook
            Must be inside AuthProvider so it can access user state */}
        <WebSocketProvider>
          {/* StellarWalletProvider: makes wallet state available via useStellarWallet() hook
              Must be inside BrowserRouter so it can call useNavigate() */}
          <StellarWalletProvider>
            {/* Main layout wrapper: light background for markets, dark for other pages */}
            <div className="min-h-screen bg-white text-gray-900 flex flex-col">
            {/* Navbar: always visible at the top, shows wallet status and navigation links */}
            <Navbar />

            {/* Main content area: flex-1 makes it fill remaining vertical space */}
            <main className="flex-1">
              <Routes>
                {/* ---- PUBLIC ROUTES ---- */}
                {/* Anyone can access these without connecting a wallet */}

                {/* Home/landing page */}
                <Route path="/" element={<Home />} />

                {/* Wallet connection page - redirects to /social-login after connecting */}
                <Route path="/login" element={<Login />} />

                {/* Traditional registration (username/password) */}
                <Route path="/register" element={<Register />} />

                {/* Post-wallet-connection onboarding/profile setup */}
                <Route path="/social-login" element={<SocialLogin />} />

                {/* Help and FAQ - /help, /faq, or /help/specific-topic */}
                <Route path="/help" element={<Help />} />
                <Route path="/faq" element={<Help />} />
                <Route path="/help/:topic" element={<Help />} />

                {/* Legal pages */}
                <Route path="/terms" element={<Terms />} />
                <Route path="/privacy" element={<Privacy />} />

                {/* Browse all prediction markets */}
                <Route path="/markets" element={<Markets />} />

                {/* Single market detail page - :id is the market ID */}
                <Route path="/markets/:id" element={<MarketDetail />} />

                {/* P2P Betting Dashboard */}
                <Route path="/bets" element={<BetDashboard />} />

                {/* P2P Bet Detail Page - :id is the bet ID */}
                <Route path="/bet/:id" element={<BetDetailPage />} />

                {/* Top traders leaderboard */}
                <Route path="/leaderboard" element={<Leaderboard />} />

                {/* Challenges list */}
                <Route path="/challenges" element={<Challenges />} />

                {/* ---- PROTECTED ROUTES ---- */}
                {/* These require wallet connection + valid JWT token.
                    ProtectedRoute redirects to /login if not authenticated. */}

                {/* User's prediction market positions */}
                <Route path="/portfolio" element={<ProtectedRoute><Portfolio /></ProtectedRoute>} />

                {/* Wallet management (balance, transactions, deposit) */}
                <Route path="/wallet" element={<ProtectedRoute><Wallet /></ProtectedRoute>} />

                {/* User notifications */}
                <Route path="/notifications" element={<ProtectedRoute><Notifications /></ProtectedRoute>} />

                {/* User profile page */}
                <Route path="/profile" element={<ProtectedRoute><Profile /></ProtectedRoute>} />
              </Routes>
            </main>

            {/* Footer: always visible at the bottom */}
            <Footer />
          </div>

          {/* Toaster: renders toast popups outside the main layout flow
              Positioned fixed on screen, shows success/error/warning messages */}
          <Toaster />
        </StellarWalletProvider>
      </WebSocketProvider>
    </AuthProvider>
  </BrowserRouter>
)
} // end App
