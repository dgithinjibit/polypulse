/**
 * ============================================================
 * FILE: StellarWalletContext.tsx
 * PURPOSE: React Context that manages the Stellar wallet state for the entire app.
 *          This is the "wallet state manager" - it:
 *            1. Connects/disconnects the Freighter wallet
 *            2. Authenticates with the PolyPulse backend after wallet connection
 *            3. Fetches and caches the wallet's XLM balance
 *            4. Persists wallet state across page refreshes via localStorage
 *            5. Exposes wallet state and actions to all child components via hooks
 *
 * PATTERN: React Context + Provider pattern
 *   - StellarWalletContext: the context object (holds the shape of the data)
 *   - StellarWalletProvider: the component that manages state and provides it
 *   - useStellarWallet: the hook that components use to access the context
 *
 * AUTHENTICATION FLOW:
 *   1. User clicks "Connect Wallet"
 *   2. connectWallet() calls stellar.connectWallet() - Freighter popup
 *   3. User approves - we get their public key
 *   4. authenticateWithBackend() gets a nonce from backend
 *   5. User signs the nonce message with their wallet
 *   6. Backend verifies the signature - returns JWT tokens
 *   7. Tokens stored in localStorage, user is now authenticated
 *
 * JUNIOR DEV NOTE:
 *   This context is provided at the App level (in App.tsx), so ALL components
 *   can access wallet state using: const { publicKey, isConnected } = useStellarWallet()
 * ============================================================
 */

// React hooks and types needed for context creation and state management
import React, { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react'

// useNavigate: programmatic navigation (redirect after wallet connect)
import { useNavigate } from 'react-router-dom'

// stellar: the StellarHelper singleton for blockchain operations
import { stellar } from '../lib/stellar-helper'

// rustApiClient: the Axios HTTP client configured for the PolyPulse backend
import rustApiClient from '../config/api'

// handleError/handleSuccess: show toast notifications for errors and successes
import { handleError, handleSuccess } from '../lib/error-handler'

// ============================================================
// TYPE: Asset
// PURPOSE: Represents a non-native Stellar token (e.g., USDC) in the wallet balance.
// ============================================================
interface Asset {
  code: string    // Token symbol e.g., 'USDC'
  issuer: string  // The Stellar account that issued this token
  balance: string // Current balance as string e.g., '50.0000000'
} // end Asset interface

// ============================================================
// TYPE: StellarWalletContextValue
// PURPOSE: Defines the shape of data and functions exposed by this context.
//          Every component that calls useStellarWallet() gets these values.
// ============================================================
interface StellarWalletContextValue {
  publicKey: string | null          // The connected wallet's public key, or null if not connected
  isConnected: boolean              // Derived from publicKey: true if publicKey is not null
  isLoading: boolean                // True while wallet connection is in progress
  isAuthenticating: boolean         // True while backend authentication is in progress
  isLoadingBalance: boolean         // True while balance is being fetched
  balance: { xlm: string; assets: Asset[] } | null  // Current wallet balance, null if not loaded
  connectWallet: () => Promise<void>   // Initiates wallet connection + backend auth flow
  disconnect: () => Promise<void>      // Disconnects wallet and clears all auth state
  refreshBalance: () => Promise<void>  // Re-fetches the wallet balance from Horizon
} // end StellarWalletContextValue interface

// ============================================================
// CONTEXT CREATION
// PURPOSE: Creates the React context with null as default.
//          null means "no provider found" - useStellarWallet() will throw if used outside provider.
// ============================================================
const StellarWalletContext = createContext<StellarWalletContextValue | null>(null)

// ============================================================
// COMPONENT: StellarWalletProvider
// PURPOSE: Wraps the app (or part of it) and provides wallet state to all children.
//          Manages all wallet-related state with useState hooks.
//          Placed in App.tsx to wrap the entire application.
// PARAM children: All child components that will have access to this context
// ============================================================
export function StellarWalletProvider({ children }: { children: ReactNode }) {
  // The connected wallet's public key. null = not connected.
  const [publicKey, setPublicKey] = useState<string | null>(null)

  // True while stellar.connectWallet() is running (shows loading spinner on button)
  const [isLoading, setIsLoading] = useState(false)

  // True while backend authentication (nonce + signature flow) is running
  const [isAuthenticating, setIsAuthenticating] = useState(false)

  // True while balance is being fetched from Horizon
  const [isLoadingBalance, setIsLoadingBalance] = useState(false)

  // The wallet's current balance. null until first fetch.
  const [balance, setBalance] = useState<{ xlm: string; assets: Asset[] } | null>(null)

  // useNavigate: used to redirect user after successful wallet connection
  const navigate = useNavigate()

  // ============================================================
  // EFFECT: Restore wallet state on app startup
  // PURPOSE: When the app loads, check if the user was previously connected.
  //          If we have both a wallet address AND a valid JWT token in localStorage,
  //          restore the connected state without requiring the user to reconnect.
  // RUNS: Once on component mount (empty dependency array)
  // ============================================================
  useEffect(() => {
    // Check localStorage for previously stored wallet address and JWT token
    const storedAddress = localStorage.getItem('wallet_address')
    const accessToken = localStorage.getItem('access_token')

    // Only restore state if BOTH are present - having one without the other means
    // the session is incomplete and the user should reconnect
    if (storedAddress && accessToken) {
      setPublicKey(storedAddress)
      // Fetch the current balance for the restored wallet
      refreshBalance()
    } // end if stored session
  }, []) // end useEffect - runs once on mount

  // ============================================================
  // FUNCTION: refreshBalance
  // PURPOSE: Fetches the current XLM and token balance from the Stellar network.
  //          Called after wallet connection and can be called manually to update balance.
  // WRAPPED IN useCallback: prevents unnecessary re-creation on every render.
  //   The function only changes if publicKey changes.
  // ============================================================
  const refreshBalance = useCallback(async () => {
    // Can't fetch balance without a connected wallet
    if (!publicKey) return

    // Show loading state while fetching
    setIsLoadingBalance(true)
    try {
      // Fetch balance from Horizon via stellar helper
      const balanceData = await stellar.getBalance(publicKey)
      // Update state with fresh balance data
      setBalance(balanceData)
    } catch (error) {
      // Balance fetch failed (e.g., network error, account not found)
      console.error('Failed to refresh balance:', error)
      // Show error toast with retry option
      handleError(error, {
        title: 'Balance Refresh Failed',
        onRetry: () => refreshBalance(),
      })
    } finally {
      // Always clear loading state, even if fetch failed
      setIsLoadingBalance(false)
    } // end try/catch/finally
  }, [publicKey]) // end refreshBalance - recreate when publicKey changes

  // ============================================================
  // FUNCTION: authenticateWithBackend
  // PURPOSE: Authenticates the connected wallet with the PolyPulse backend.
  //          Uses a challenge-response pattern:
  //            1. Get a unique nonce from the backend (prevents replay attacks)
  //            2. Build a message containing the nonce
  //            3. Sign the message with the wallet (proves wallet ownership)
  //            4. Send the signature to the backend for verification
  //            5. Receive JWT tokens on success
  // PARAM address: The wallet's public key to authenticate
  // RETURNS: Promise<true> on success
  // THROWS: Re-throws any error after showing an error toast
  // CALLED BY: connectWallet() after getting the public key
  // ============================================================
  const authenticateWithBackend = useCallback(async (address: string) => {
    // Show authenticating state (different from isLoading - this is the backend auth step)
    setIsAuthenticating(true)
    try {
      // STEP 1: Request a nonce from the backend.
      // A nonce is a one-time random string that prevents replay attacks.
      // The backend stores this nonce and expects it back in the signed message.
      const nonceRes = await rustApiClient.post<{ nonce: string }>('/api/v1/auth/stellar-nonce', {
        public_key: address
      })
      const { nonce } = nonceRes.data

      // STEP 2: Build the authentication message.
      // This exact format must match what the backend expects to verify.
      const message = `PolyPulse Login\nAddress: ${address}\nNonce: ${nonce}`

      // STEP 3: Sign the message with the wallet.
      // stellar.signAuthMessage() builds a Stellar transaction containing the message
      // and asks Freighter to sign it. The signed XDR is our "signature".
      const { signature } = await stellar.signAuthMessage(message)

      // STEP 4: Send the signature to the backend for verification.
      // The backend will verify that the signature was created by the wallet at `address`.
      const authRes = await rustApiClient.post<{ access: string; refresh: string }>(
        '/api/v1/auth/stellar-login',
        {
          public_key: address,
          signature,
          message
        }
      )

      // STEP 5: Store the JWT tokens in localStorage for future requests.
      // access_token: short-lived token for API requests (sent in Authorization header)
      // refresh_token: long-lived token used to get new access tokens when they expire
      // wallet_address: stored so we can restore session on page refresh
      localStorage.setItem('access_token', authRes.data.access)
      localStorage.setItem('refresh_token', authRes.data.refresh)
      localStorage.setItem('wallet_address', address)

      return true
    } catch (error) {
      // Authentication failed - could be network error, invalid signature, or backend error
      console.error('Authentication failed:', error)
      // Show error toast with retry option
      handleError(error, {
        title: 'Authentication Failed',
        onRetry: async () => { await authenticateWithBackend(address) },
      })
      // Re-throw so connectWallet() knows authentication failed
      throw error
    } finally {
      // Always clear authenticating state
      setIsAuthenticating(false)
    } // end try/catch/finally
  }, []) // end authenticateWithBackend

  // ============================================================
  // FUNCTION: connectWallet
  // PURPOSE: The main wallet connection flow. Called when user clicks "Connect Wallet".
  //          Orchestrates: wallet connection - backend auth - balance fetch - navigation
  // WRAPPED IN useCallback: stable reference for use in dependency arrays
  // ============================================================
  const connectWallet = useCallback(async () => {
    // Show loading state on the connect button
    setIsLoading(true)
    try {
      // STEP 1: Connect to Freighter wallet - shows browser extension popup
      const address = await stellar.connectWallet()

      // STEP 2: Store the public key in React state
      setPublicKey(address)

      // STEP 3: Authenticate with the PolyPulse backend using the wallet signature
      await authenticateWithBackend(address)

      // STEP 4: Fetch the wallet's current balance
      const balanceData = await stellar.getBalance(address)
      setBalance(balanceData)

      // STEP 5: Show success toast notification
      handleSuccess('Wallet Connected', `Connected to ${stellar.formatAddress(address)}`)

      // STEP 6: Navigate to the onboarding/social login page
      navigate('/social-login')
    } catch (error: any) {
      // Connection failed at some step - clear the public key
      console.error('Wallet connection failed:', error)
      setPublicKey(null)
      // Show error toast with retry option
      handleError(error, {
        onRetry: () => connectWallet(),
      })
      // Re-throw so callers (e.g., Login page) can handle it too
      throw error
    } finally {
      // Always clear loading state
      setIsLoading(false)
    } // end try/catch/finally
  }, [authenticateWithBackend, navigate]) // end connectWallet

  // ============================================================
  // FUNCTION: disconnect
  // PURPOSE: Disconnects the wallet and clears all authentication state.
  //          Removes JWT tokens and wallet address from localStorage.
  //          Resets all wallet state to initial values.
  // ============================================================
  const disconnect = useCallback(async () => {
    try {
      // Tell the stellar helper to clear its cached public key
      await stellar.disconnect()

      // Remove all auth-related data from localStorage
      localStorage.removeItem('access_token')
      localStorage.removeItem('refresh_token')
      localStorage.removeItem('wallet_address')

      // Reset React state to initial values
      setPublicKey(null)
      setBalance(null)

      // Show success toast
      handleSuccess('Wallet Disconnected', 'Your wallet has been disconnected successfully.')
    } catch (error) {
      // Disconnect shouldn't fail, but handle it gracefully just in case
      console.error('Disconnect failed:', error)
      handleError(error, {
        title: 'Disconnect Failed',
      })
    } // end try/catch
  }, []) // end disconnect

  // ============================================================
  // PROVIDER RENDER
  // PURPOSE: Provides the wallet state and actions to all child components.
  //          Any component inside StellarWalletProvider can call useStellarWallet()
  //          to access these values.
  // ============================================================
  return (
    <StellarWalletContext.Provider
      value={{
        publicKey,
        isConnected: !!publicKey,  // Derived boolean: true if publicKey is not null/empty
        isLoading,
        isAuthenticating,
        isLoadingBalance,
        balance,
        connectWallet,
        disconnect,
        refreshBalance,
      }}
    >
      {/* Render all child components with access to this context */}
      {children}
    </StellarWalletContext.Provider>
  )
} // end StellarWalletProvider

// ============================================================
// HOOK: useStellarWallet
// PURPOSE: Custom hook for consuming the StellarWalletContext.
//          Throws a helpful error if used outside the provider.
// RETURNS: StellarWalletContextValue - all wallet state and actions
// USAGE: const { publicKey, isConnected, connectWallet } = useStellarWallet()
// ============================================================
export function useStellarWallet(): StellarWalletContextValue {
  const ctx = useContext(StellarWalletContext)

  // If ctx is null, this hook was called outside of StellarWalletProvider
  // This is a developer error - throw a clear message
  if (!ctx) throw new Error('useStellarWallet must be used within StellarWalletProvider')

  return ctx
} // end useStellarWallet
