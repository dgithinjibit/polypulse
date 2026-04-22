/**
 * ============================================================
 * FILE: AuthContext.tsx
 * PURPOSE: React Context for traditional username/password authentication.
 *          Manages the logged-in user's profile data and JWT tokens.
 *          NOTE: For Stellar wallet users, authentication is handled by
 *          StellarWalletContext. This context handles the legacy
 *          username/password login flow (Register/Login pages).
 *
 * PROVIDES:
 *   - user: The current user's profile data (id, username, email, balance)
 *   - loading: True while checking if user is already logged in on startup
 *   - login: Authenticates with username + password, stores JWT tokens
 *   - logout: Clears JWT tokens and user state
 *   - refreshUser: Re-fetches the user profile from the backend
 *
 * JUNIOR DEV NOTE:
 *   On app startup, this context checks localStorage for an existing access_token.
 *   If found, it fetches the user profile to restore the logged-in state.
 *   This prevents users from being logged out on page refresh.
 * ============================================================
 */

// React hooks and types for context creation
import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react'

// rustApiClient: Axios HTTP client configured with base URL and JWT interceptors
import rustApiClient from '../config/api'

// ============================================================
// TYPE: User
// PURPOSE: Represents a PolyPulse user profile returned by the backend.
//          The [key: string]: unknown allows for additional fields the backend may return.
// ============================================================
interface User {
  id: number          // Unique user ID in the database
  username: string    // Display name
  email: string       // Email address
  balance?: number    // Optional: user's platform balance (not wallet balance)
  [key: string]: unknown  // Allow any additional fields from the backend
} // end User interface

// ============================================================
// TYPE: AuthContextValue
// PURPOSE: Defines what this context exposes to consuming components.
// ============================================================
interface AuthContextValue {
  user: User | null                                          // Current user or null if not logged in
  loading: boolean                                           // True while fetching user profile on startup
  login: (username: string, password: string) => Promise<unknown>  // Login with credentials
  logout: () => void                                         // Clear auth state
  refreshUser: () => Promise<void>                           // Re-fetch user profile
} // end AuthContextValue interface

// Create the context with null default (throws if used outside provider)
const AuthContext = createContext<AuthContextValue | null>(null)

// ============================================================
// COMPONENT: AuthProvider
// PURPOSE: Provides authentication state to all child components.
//          Placed in App.tsx to wrap the entire application.
// PARAM children: Child components that will have access to auth state
// ============================================================
export function AuthProvider({ children }: { children: ReactNode }) {
  // The current logged-in user's profile. null = not logged in.
  const [user, setUser] = useState<User | null>(null)

  // True while the initial profile fetch is running on app startup.
  // Prevents showing "not logged in" state before we've checked localStorage.
  const [loading, setLoading] = useState(true)

  // ============================================================
  // FUNCTION: fetchProfile
  // PURPOSE: Fetches the current user's profile from the backend.
  //          Called on startup (if token exists) and after login.
  //          Uses the JWT token from localStorage (injected by apiClient interceptor).
  // ============================================================
  const fetchProfile = useCallback(async () => {
    try {
      // GET /api/v1/users/me - returns the current user's data
      // The JWT token is automatically added by rustApiClient's request interceptor
      const data = await rustApiClient.get<User>('/api/v1/users/me')
      setUser(data.data)
    } catch {
      // Profile fetch failed - token may be expired or invalid
      // Clear user state (treat as logged out)
      setUser(null)
    } finally {
      // Always clear loading state after the check completes
      setLoading(false)
    } // end try/catch/finally
  }, []) // end fetchProfile

  // ============================================================
  // EFFECT: Check for existing session on app startup
  // PURPOSE: If there's an access_token in localStorage, try to restore the session
  //          by fetching the user profile. If no token, just set loading to false.
  // RUNS: Once on component mount
  // ============================================================
  useEffect(() => {
    if (localStorage.getItem('access_token')) {
      // Token exists - try to fetch the user profile to restore session
      fetchProfile()
    } else {
      // No token - user is not logged in, stop loading immediately
      setLoading(false)
    } // end if token check
  }, [fetchProfile]) // end useEffect

  // ============================================================
  // FUNCTION: login
  // PURPOSE: Authenticates a user with username and password.
  //          Stores JWT tokens in localStorage on success.
  //          Then fetches the user profile to populate the user state.
  // PARAM username: The user's username
  // PARAM password: The user's password
  // RETURNS: The JWT token data from the backend
  // THROWS: If credentials are invalid or network error occurs
  // ============================================================
  const login = async (username: string, password: string) => {
    // Generate a simple device fingerprint for security tracking
    // This helps the backend detect suspicious login patterns
    const fp = 'browser-' + navigator.userAgent.length

    // POST /api/v1/auth/login with credentials
    // Returns { access: string, refresh: string } JWT tokens
    const data = await rustApiClient.post<{ access: string; refresh: string }>(
      '/api/v1/auth/login',
      { username, password },
      { headers: { 'X-Device-Fingerprint': fp } }  // Send device fingerprint header
    )

    // Store JWT tokens in localStorage for use in future requests
    localStorage.setItem('access_token', data.data.access)
    localStorage.setItem('refresh_token', data.data.refresh)

    // Fetch and store the user profile now that we have a valid token
    await fetchProfile()

    // Return the token data in case the caller needs it
    return data.data
  } // end login

  // ============================================================
  // FUNCTION: logout
  // PURPOSE: Clears all authentication state.
  //          Removes JWT tokens from localStorage and clears user state.
  //          NOTE: Does not call the backend logout endpoint (stateless JWT).
  // ============================================================
  const logout = () => {
    // Remove JWT tokens - future API requests will fail with 401
    localStorage.removeItem('access_token')
    localStorage.removeItem('refresh_token')

    // Clear user state - components will see user as null (logged out)
    setUser(null)
  } // end logout

  // ============================================================
  // PROVIDER RENDER
  // PURPOSE: Provides auth state and actions to all child components.
  // ============================================================
  return (
    <AuthContext.Provider value={{ user, loading, login, logout, refreshUser: fetchProfile }}>
      {children}
    </AuthContext.Provider>
  )
} // end AuthProvider

// ============================================================
// HOOK: useAuth
// PURPOSE: Custom hook for consuming the AuthContext.
//          Throws a helpful error if used outside the provider.
// RETURNS: AuthContextValue - user state and auth actions
// USAGE: const { user, login, logout } = useAuth()
// ============================================================
export const useAuth = (): AuthContextValue => {
  const ctx = useContext(AuthContext)

  // If ctx is null, this hook was called outside of AuthProvider
  if (!ctx) throw new Error('useAuth must be used within AuthProvider')

  return ctx
} // end useAuth
