/**
 * ============================================================
 * FILE: config/api.ts
 * PURPOSE: Axios HTTP client configured for the PolyPulse Rust backend.
 *          This is the PRIMARY API client used throughout the app.
 *          Handles:
 *            - Direct connection to the Rust backend (not proxied)
 *            - Automatic JWT token injection on every request
 *            - Automatic token refresh on 401 (with request queuing)
 *            - Redirect to /login if refresh also fails
 *
 * DIFFERENCE FROM services/api.ts:
 *   services/api.ts - Python/Django backend (legacy), simple 401 redirect
 *   config/api.ts   - Rust backend (current), smart token refresh with queue
 *
 * TOKEN REFRESH FLOW:
 *   1. Request fails with 401 (access token expired)
 *   2. We try to get a new access token using the refresh token
 *   3. While refreshing, any other 401 requests are queued
 *   4. Once refresh succeeds, all queued requests are retried with new token
 *   5. If refresh fails, clear all tokens and redirect to /login
 *
 * JUNIOR DEV NOTE:
 *   The "failed queue" pattern prevents multiple simultaneous refresh attempts.
 *   Without it, if 5 requests fail at once, we'd make 5 refresh calls.
 *   With it, only 1 refresh happens and the other 4 wait for it.
 * ============================================================
 */

// Import Axios and its TypeScript types
import axios, { AxiosInstance, InternalAxiosRequestConfig, AxiosResponse } from 'axios'

// ============================================================
// BACKEND URL CONFIGURATION
// PURPOSE: Read the Rust backend URL from environment variables.
//          In development: set VITE_API_URL in .env (e.g., http://localhost:8000)
//          In production: set VITE_API_URL to the deployed backend URL
//          Falls back to localhost:8000 if not set.
// ============================================================
const RUST_BACKEND_URL = import.meta.env.VITE_API_URL || 'http://localhost:8000'

// ============================================================
// CREATE AXIOS INSTANCE
// PURPOSE: Creates a configured Axios instance pointing at the Rust backend.
//          timeout: 10000ms = 10 seconds before a request is considered failed.
// ============================================================
const rustApiClient: AxiosInstance = axios.create({
  baseURL: RUST_BACKEND_URL,  // Direct URL to Rust backend
  timeout: 60000,             // 60 second timeout for all requests (handles Render cold starts)
  headers: {
    'Content-Type': 'application/json',
  },
})

// ============================================================
// REQUEST INTERCEPTOR: Inject JWT Token
// PURPOSE: Automatically adds the JWT access token to every outgoing request.
//          Same pattern as services/api.ts - avoids manual header setting everywhere.
// ============================================================
rustApiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    // Get the current access token from localStorage
    const token = localStorage.getItem('access_token')

    // Inject the token into the Authorization header if it exists
    if (token && config.headers) {
      config.headers.Authorization = `Bearer ${token}`
    } // end token injection

    return config
  },
  (error) => Promise.reject(error)
) // end request interceptor

// ============================================================
// TOKEN REFRESH STATE
// PURPOSE: These variables track the state of an in-progress token refresh.
//          They are module-level (not inside a function) so they persist
//          across multiple interceptor calls.
// ============================================================

// True while a token refresh request is in progress.
// Prevents multiple simultaneous refresh attempts.
let isRefreshing = false

// Queue of requests that failed with 401 while a refresh was in progress.
// Each entry has resolve/reject functions to retry or fail the original request.
let failedQueue: Array<{
  resolve: (value: unknown) => void
  reject: (reason?: unknown) => void
}> = []

// ============================================================
// FUNCTION: processQueue
// PURPOSE: After a token refresh completes (success or failure),
//          process all queued requests.
//          If refresh succeeded: resolve each queued request with the new token.
//          If refresh failed: reject each queued request with the error.
// PARAM error: The refresh error (null if refresh succeeded)
// PARAM token: The new access token (null if refresh failed)
// ============================================================
function processQueue(error: unknown, token: string | null = null) {
  // Process each queued request
  failedQueue.forEach(({ resolve, reject }) => {
    if (error) {
      // Refresh failed - reject the queued request with the error
      reject(error)
    } else {
      // Refresh succeeded - resolve with the new token so the request can retry
      resolve(token)
    } // end if error
  }) // end forEach

  // Clear the queue after processing
  failedQueue = []
} // end processQueue

// ============================================================
// RESPONSE INTERCEPTOR: Handle 401 with Token Refresh
// PURPOSE: When a request fails with 401 (Unauthorized):
//          1. If we have a refresh token, try to get a new access token
//          2. Retry the original request with the new token
//          3. If refresh fails, clear all auth state and redirect to /login
// ============================================================
rustApiClient.interceptors.response.use(
  // Success handler: pass through successful responses unchanged
  (response: AxiosResponse) => response,

  // Error handler: handle 401 with token refresh logic
  async (error) => {
    // Get the original request config so we can retry it
    const originalRequest = error.config

    // Only handle 401 errors, and only if we haven't already retried this request.
    // _retry flag prevents infinite retry loops.
    if (error.response?.status !== 401 || originalRequest._retry) {
      return Promise.reject(error)
    } // end non-401 check

    // Check if we have a refresh token to use
    const refreshToken = localStorage.getItem('refresh_token')
    if (!refreshToken) {
      // No refresh token available - can't refresh, must re-login
      localStorage.removeItem('access_token')
      localStorage.removeItem('refresh_token')
      localStorage.removeItem('wallet_address')
      window.location.href = '/login'
      return Promise.reject(error)
    } // end no refresh token check

    // If a refresh is already in progress, queue this request to retry after refresh
    if (isRefreshing) {
      // Return a promise that will resolve/reject when the refresh completes
      return new Promise((resolve, reject) => {
        // Add to the queue - processQueue() will call resolve/reject later
        failedQueue.push({ resolve, reject })
      })
        .then((token) => {
          // Refresh succeeded - retry the original request with the new token
          originalRequest.headers.Authorization = `Bearer ${token}`
          return rustApiClient(originalRequest)
        })
        .catch((err) => Promise.reject(err))
    } // end isRefreshing check

    // Mark this request as retried to prevent infinite loops
    originalRequest._retry = true

    // Mark refresh as in progress to queue any other 401s that come in
    isRefreshing = true

    try {
      // Attempt to get a new access token using the refresh token
      // Note: using plain axios (not rustApiClient) to avoid interceptor loops
      const response = await axios.post(`${RUST_BACKEND_URL}/api/v1/auth/refresh`, {
        refresh: refreshToken,
      })

      // Extract the new tokens from the response
      const { access, refresh } = response.data

      // Store the new tokens in localStorage
      localStorage.setItem('access_token', access)
      localStorage.setItem('refresh_token', refresh)

      // Update the default Authorization header for future requests
      rustApiClient.defaults.headers.common.Authorization = `Bearer ${access}`

      // Update the original request's header with the new token
      originalRequest.headers.Authorization = `Bearer ${access}`

      // Resolve all queued requests with the new token
      processQueue(null, access)

      // Retry the original request that triggered the 401
      return rustApiClient(originalRequest)
    } catch (refreshError) {
      // Refresh failed (refresh token expired or invalid)
      // Reject all queued requests
      processQueue(refreshError, null)

      // Clear all auth state - user must log in again
      localStorage.removeItem('access_token')
      localStorage.removeItem('refresh_token')
      localStorage.removeItem('wallet_address')

      // Redirect to login
      window.location.href = '/login'

      return Promise.reject(refreshError)
    } finally {
      // Always clear the refreshing flag when done
      isRefreshing = false
    } // end try/catch/finally
  }
) // end response interceptor

// ============================================================
// ENDPOINT CONSTANTS
// PURPOSE: Centralized URL constants for all API endpoints.
//          Using constants prevents typos and makes refactoring easier.
//          If an endpoint URL changes, update it here once.
// ============================================================

// Wager-related endpoints
export const WAGER_ENDPOINTS = {
  LIST: '/api/wagers',                              // GET: list all wagers
  CREATE: '/api/wagers',                            // POST: create a new wager
  DETAIL: (id: string) => `/api/wagers/${id}`,      // GET: single wager details
  ACCEPT: (id: string) => `/api/wagers/${id}/accept`, // POST: accept a wager
  CANCEL: (id: string) => `/api/wagers/${id}/cancel`, // POST: cancel a wager
  DISCOVER: '/api/wagers/discover',                 // GET: discover public wagers
  PORTFOLIO: '/api/wagers/portfolio',               // GET: user's wager portfolio
  YIELD: (id: string) => `/api/wagers/${id}/yield`, // GET: wager yield/returns
}

// Chat message endpoints (within wager rooms)
export const CHAT_ENDPOINTS = {
  MESSAGES: (wagerId: string) => `/api/wagers/${wagerId}/messages`, // GET: fetch messages
  SEND: (wagerId: string) => `/api/wagers/${wagerId}/messages`,     // POST: send a message
}

// Authentication endpoints
export const AUTH_ENDPOINTS = {
  VERIFY: '/api/v1/auth/login',           // POST: login with credentials
  REFRESH: '/api/v1/auth/refresh',        // POST: refresh JWT tokens
  PROFILE: '/api/v1/auth/profile',        // GET: current user profile
  LOGOUT: '/api/v1/auth/logout',          // POST: logout
  STELLAR_NONCE: '/api/v1/auth/stellar-nonce',  // POST: get nonce for wallet auth
  STELLAR_LOGIN: '/api/v1/auth/stellar-login',  // POST: authenticate with wallet signature
}

// Export the configured Rust API client as default
export default rustApiClient
