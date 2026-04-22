/**
 * ============================================================
 * FILE: ProtectedRoute.tsx
 * PURPOSE: A route guard component that prevents unauthenticated users
 *          from accessing protected pages.
 *          If the user is not connected (no wallet) or has no JWT token,
 *          they are redirected to /login.
 *          While the wallet state is loading (checking localStorage on startup),
 *          shows a loading spinner to prevent a flash of the login redirect.
 *
 * USAGE in App.tsx:
 *   <Route path="/portfolio" element={<ProtectedRoute><Portfolio /></ProtectedRoute>} />
 *
 * JUNIOR DEV NOTE:
 *   This component uses the "render props" pattern - it wraps children and
 *   decides whether to render them or redirect based on auth state.
 * ============================================================
 */

import React, { ReactNode } from 'react'

// Navigate: React Router component that performs a redirect when rendered
import { Navigate } from 'react-router-dom'

// useStellarWallet: access wallet connection state
import { useStellarWallet } from '../context/StellarWalletContext'

// ============================================================
// TYPE: ProtectedRouteProps
// PURPOSE: Defines the props this component accepts.
//          children: The page component to render if authenticated.
// ============================================================
interface ProtectedRouteProps {
  children: ReactNode  // The protected page component (e.g., <Portfolio />)
} // end ProtectedRouteProps

// ============================================================
// COMPONENT: ProtectedRoute
// PURPOSE: Checks authentication state and either renders children or redirects.
// PARAM children: The component to render if the user is authenticated
// ============================================================
export default function ProtectedRoute({ children }: ProtectedRouteProps) {
  // Get wallet connection state from context
  const { isConnected, isLoading } = useStellarWallet()

  // Also check for JWT token in localStorage as a secondary auth check.
  // A user could have isConnected=true but no token (e.g., token was manually deleted).
  // Both conditions must be true for full authentication.
  const hasTokens = !!localStorage.getItem('access_token')

  // ---- LOADING STATE ----
  // While the wallet context is initializing (checking localStorage on startup),
  // show a spinner instead of immediately redirecting.
  // Without this, users would see a flash redirect to /login on every page refresh.
  if (isLoading) {
    return (
      <div className="flex justify-center items-center h-64">
        {/* Spinning circle loading indicator */}
        <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600" />
      </div>
    )
  } // end loading check

  // ---- AUTH CHECK ----
  // If wallet is not connected OR no JWT token exists, redirect to login.
  // replace: true means the /login page replaces the current history entry,
  // so pressing "back" won't bring the user back to the protected page.
  if (!isConnected || !hasTokens) {
    return <Navigate to="/login" replace />
  } // end auth check

  // ---- AUTHENTICATED ----
  // User is connected and has a valid token - render the protected page.
  // The fragment <></> is used to avoid adding an extra DOM element.
  return <>{children}</>
} // end ProtectedRoute
