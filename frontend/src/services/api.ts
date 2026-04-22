/**
 * ============================================================
 * FILE: services/api.ts
 * PURPOSE: Axios HTTP client configured for the PolyPulse Python/Django backend.
 *          Handles:
 *            - Base URL configuration (proxied through Vite to /api)
 *            - Automatic JWT token injection on every request
 *            - Automatic redirect to /login on 401 Unauthorized responses
 *
 * NOTE: This client is for the PYTHON backend (legacy).
 *       For the Rust backend, use config/api.ts (rustApiClient).
 *
 * JUNIOR DEV NOTE:
 *   Axios interceptors are middleware functions that run before/after every request.
 *   Request interceptor: adds the JWT token to the Authorization header.
 *   Response interceptor: handles 401 errors by clearing tokens and redirecting.
 * ============================================================
 */

// Import Axios and its TypeScript types
import axios, { AxiosInstance, InternalAxiosRequestConfig, AxiosResponse } from 'axios'

// ============================================================
// CREATE AXIOS INSTANCE
// PURPOSE: Creates a configured Axios instance with default settings.
//          baseURL '/api' is proxied by Vite to the actual backend URL
//          (configured in vite.config.ts proxy settings).
// ============================================================
const apiClient: AxiosInstance = axios.create({
  baseURL: '/api',  // Vite proxies /api/* to the backend server
  headers: {
    'Content-Type': 'application/json',  // All requests send/expect JSON
  },
})

// ============================================================
// REQUEST INTERCEPTOR: Inject JWT Token
// PURPOSE: Automatically adds the JWT access token to every outgoing request.
//          This means we don't have to manually add the Authorization header
//          in every API call throughout the codebase.
// HOW IT WORKS:
//   Before each request, this function runs and checks localStorage for a token.
//   If found, it adds: Authorization: Bearer <token> to the request headers.
// ============================================================
apiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    // Get the JWT access token from localStorage
    const token = localStorage.getItem('access_token')

    // If token exists and headers object is available, inject it
    if (token && config.headers) {
      config.headers.Authorization = `Bearer ${token}`
    } // end token injection

    // Return the modified config to proceed with the request
    return config
  },
  // If the request setup itself fails, reject the promise
  (error) => Promise.reject(error)
) // end request interceptor

// ============================================================
// RESPONSE INTERCEPTOR: Handle 401 Unauthorized
// PURPOSE: When the backend returns 401 (token expired or invalid),
//          automatically clear the stored tokens and redirect to /login.
//          This prevents the user from being stuck in a broken auth state.
// NOTE: This client does NOT attempt token refresh (unlike config/api.ts).
//       On 401, it immediately clears state and redirects.
// ============================================================
apiClient.interceptors.response.use(
  // Success handler: pass through successful responses unchanged
  (response: AxiosResponse) => response,

  // Error handler: check for 401 and handle it
  (error) => {
    if (error.response?.status === 401) {
      // Token is expired or invalid - clear all auth state
      localStorage.removeItem('access_token')
      localStorage.removeItem('refresh_token')

      // Redirect to login page so user can re-authenticate
      window.location.href = '/login'
    } // end 401 check

    // Re-reject the error so calling code can still handle it if needed
    return Promise.reject(error)
  }
) // end response interceptor

// Export the configured client as the default export
export default apiClient
