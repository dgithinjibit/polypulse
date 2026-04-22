/**
 * Unit tests for StellarWalletContext
 * Tests wallet state management, authentication flow, and state persistence.
 * Mocks: stellar-helper singleton, apiClient, react-router-dom, error-handler
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, act, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import React from 'react'
import * as fc from 'fast-check'

// ── Mocks ──────────────────────────────────────────────────────────────────

const mockNavigate = vi.fn()
vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}))

const mockConnectWallet = vi.fn()
const mockGetBalance = vi.fn()
const mockSignAuthMessage = vi.fn()
const mockDisconnect = vi.fn()
const mockFormatAddress = vi.fn((addr: string) => `${addr.slice(0, 4)}...${addr.slice(-4)}`)

vi.mock('../lib/stellar-helper', () => ({
  stellar: {
    connectWallet: mockConnectWallet,
    getBalance: mockGetBalance,
    signAuthMessage: mockSignAuthMessage,
    disconnect: mockDisconnect,
    formatAddress: mockFormatAddress,
  },
}))

const mockApiPost = vi.fn()
vi.mock('../services/api', () => ({
  default: { post: mockApiPost },
}))

vi.mock('../lib/error-handler', () => ({
  handleError: vi.fn(),
  handleSuccess: vi.fn(),
}))

import { StellarWalletProvider, useStellarWallet } from '../context/StellarWalletContext'

// ── Helpers ────────────────────────────────────────────────────────────────

const MOCK_KEY = 'GBTEST1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCDE'
const MOCK_BALANCE = { xlm: '100.0000000', assets: [] }
const MOCK_TOKENS = { access: 'access_jwt', refresh: 'refresh_jwt' }

/** Renders a component inside StellarWalletProvider */
function renderWithProvider(ui: React.ReactElement) {
  return render(<StellarWalletProvider>{ui}</StellarWalletProvider>)
}

/** A simple consumer component that exposes context values via data-testid */
function WalletConsumer() {
  const { publicKey, isConnected, isLoading, balance, connectWallet, disconnect } =
    useStellarWallet()
  return (
    <div>
      <span data-testid="publicKey">{publicKey ?? 'null'}</span>
      <span data-testid="isConnected">{String(isConnected)}</span>
      <span data-testid="isLoading">{String(isLoading)}</span>
      <span data-testid="balance">{balance ? balance.xlm : 'null'}</span>
      <button onClick={connectWallet}>Connect</button>
      <button onClick={disconnect}>Disconnect</button>
    </div>
  )
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('StellarWalletContext', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mockGetBalance.mockResolvedValue(MOCK_BALANCE)
    mockDisconnect.mockResolvedValue(true)
  })

  afterEach(() => {
    localStorage.clear()
  })

  // ── Initialization ───────────────────────────────────────────────────────

  describe('initialization', () => {
    it('starts disconnected with null publicKey', () => {
      renderWithProvider(<WalletConsumer />)

      expect(screen.getByTestId('publicKey').textContent).toBe('null')
      expect(screen.getByTestId('isConnected').textContent).toBe('false')
      expect(screen.getByTestId('balance').textContent).toBe('null')
    })

    it('restores session from localStorage when both tokens exist', async () => {
      localStorage.setItem('wallet_address', MOCK_KEY)
      localStorage.setItem('access_token', 'stored_token')

      renderWithProvider(<WalletConsumer />)

      await waitFor(() => {
        expect(screen.getByTestId('publicKey').textContent).toBe(MOCK_KEY)
        expect(screen.getByTestId('isConnected').textContent).toBe('true')
      })
    })

    it('does NOT restore session when only wallet_address is stored (no token)', async () => {
      localStorage.setItem('wallet_address', MOCK_KEY)
      // No access_token

      renderWithProvider(<WalletConsumer />)

      await waitFor(() => {
        expect(screen.getByTestId('publicKey').textContent).toBe('null')
      })
    })

    it('does NOT restore session when only access_token is stored (no address)', async () => {
      localStorage.setItem('access_token', 'stored_token')
      // No wallet_address

      renderWithProvider(<WalletConsumer />)

      await waitFor(() => {
        expect(screen.getByTestId('publicKey').textContent).toBe('null')
      })
    })
  })

  // ── connectWallet ────────────────────────────────────────────────────────

  describe('connectWallet()', () => {
    beforeEach(() => {
      mockConnectWallet.mockResolvedValue(MOCK_KEY)
      mockApiPost
        .mockResolvedValueOnce({ data: { nonce: 'test_nonce_123' } })
        .mockResolvedValueOnce({ data: MOCK_TOKENS })
      mockSignAuthMessage.mockResolvedValue({ signature: 'SIGNED_XDR', publicKey: MOCK_KEY })
    })

    it('sets publicKey and isConnected after successful connection', async () => {
      renderWithProvider(<WalletConsumer />)

      await act(async () => {
        await userEvent.click(screen.getByText('Connect'))
      })

      expect(screen.getByTestId('publicKey').textContent).toBe(MOCK_KEY)
      expect(screen.getByTestId('isConnected').textContent).toBe('true')
    })

    it('stores JWT tokens in localStorage after successful auth', async () => {
      renderWithProvider(<WalletConsumer />)

      await act(async () => {
        await userEvent.click(screen.getByText('Connect'))
      })

      expect(localStorage.getItem('access_token')).toBe(MOCK_TOKENS.access)
      expect(localStorage.getItem('refresh_token')).toBe(MOCK_TOKENS.refresh)
      expect(localStorage.getItem('wallet_address')).toBe(MOCK_KEY)
    })

    it('navigates to /social-login after successful connection', async () => {
      renderWithProvider(<WalletConsumer />)

      await act(async () => {
        await userEvent.click(screen.getByText('Connect'))
      })

      expect(mockNavigate).toHaveBeenCalledWith('/social-login')
    })

    it('calls stellar-nonce then stellar-login endpoints in order', async () => {
      renderWithProvider(<WalletConsumer />)

      await act(async () => {
        await userEvent.click(screen.getByText('Connect'))
      })

      expect(mockApiPost).toHaveBeenNthCalledWith(1, '/v1/auth/stellar-nonce', {
        public_key: MOCK_KEY,
      })
      expect(mockApiPost).toHaveBeenNthCalledWith(2, '/v1/auth/stellar-login', {
        public_key: MOCK_KEY,
        signature: 'SIGNED_XDR',
        message: expect.stringContaining('test_nonce_123'),
      })
    })

    it('clears publicKey and does NOT navigate on connection failure', async () => {
      mockConnectWallet.mockRejectedValue(new Error('User rejected'))

      renderWithProvider(<WalletConsumer />)

      await act(async () => {
        try {
          await userEvent.click(screen.getByText('Connect'))
        } catch {}
      })

      expect(screen.getByTestId('publicKey').textContent).toBe('null')
      expect(mockNavigate).not.toHaveBeenCalled()
    })

    it('clears publicKey on backend auth failure', async () => {
      mockApiPost.mockRejectedValue(new Error('Backend error'))

      renderWithProvider(<WalletConsumer />)

      await act(async () => {
        try {
          await userEvent.click(screen.getByText('Connect'))
        } catch {}
      })

      expect(screen.getByTestId('publicKey').textContent).toBe('null')
    })
  })

  // ── disconnect ───────────────────────────────────────────────────────────

  describe('disconnect()', () => {
    beforeEach(async () => {
      // Pre-populate localStorage as if user was connected
      localStorage.setItem('wallet_address', MOCK_KEY)
      localStorage.setItem('access_token', 'access_jwt')
      localStorage.setItem('refresh_token', 'refresh_jwt')
    })

    it('clears publicKey and isConnected after disconnect', async () => {
      renderWithProvider(<WalletConsumer />)

      // Wait for session restore
      await waitFor(() => {
        expect(screen.getByTestId('isConnected').textContent).toBe('true')
      })

      await act(async () => {
        await userEvent.click(screen.getByText('Disconnect'))
      })

      expect(screen.getByTestId('publicKey').textContent).toBe('null')
      expect(screen.getByTestId('isConnected').textContent).toBe('false')
    })

    it('removes all auth tokens from localStorage after disconnect', async () => {
      renderWithProvider(<WalletConsumer />)

      await waitFor(() => {
        expect(screen.getByTestId('isConnected').textContent).toBe('true')
      })

      await act(async () => {
        await userEvent.click(screen.getByText('Disconnect'))
      })

      expect(localStorage.getItem('access_token')).toBeNull()
      expect(localStorage.getItem('refresh_token')).toBeNull()
      expect(localStorage.getItem('wallet_address')).toBeNull()
    })

    it('calls stellar.disconnect()', async () => {
      renderWithProvider(<WalletConsumer />)

      await waitFor(() => {
        expect(screen.getByTestId('isConnected').textContent).toBe('true')
      })

      await act(async () => {
        await userEvent.click(screen.getByText('Disconnect'))
      })

      expect(mockDisconnect).toHaveBeenCalled()
    })
  })

  // ── useStellarWallet hook guard ──────────────────────────────────────────

  describe('useStellarWallet()', () => {
    it('throws when used outside StellarWalletProvider', () => {
      // Suppress React's error boundary console output
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

      expect(() => {
        render(
          <React.Suspense fallback={null}>
            <WalletConsumer />
          </React.Suspense>
        )
      }).toThrow('useStellarWallet must be used within StellarWalletProvider')

      spy.mockRestore()
    })
  })
})

// ── Property-Based Tests ───────────────────────────────────────────────────

describe('StellarWalletContext - Correctness Properties', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mockGetBalance.mockResolvedValue(MOCK_BALANCE)
    mockDisconnect.mockResolvedValue(true)
  })

  afterEach(() => {
    localStorage.clear()
  })

  /**
   * Property 1: isConnected ⟺ publicKey !== null
   * If isConnected is true, publicKey must be non-null, and vice versa.
   */
  it('isConnected is always true iff publicKey is non-null', async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.boolean(), // whether to simulate a connected session
        async (shouldBeConnected) => {
          localStorage.clear()
          if (shouldBeConnected) {
            localStorage.setItem('wallet_address', MOCK_KEY)
            localStorage.setItem('access_token', 'token')
          }

          let capturedState: { isConnected: boolean; publicKey: string | null } | null = null

          function StateCapture() {
            const { isConnected, publicKey } = useStellarWallet()
            capturedState = { isConnected, publicKey }
            return null
          }

          await act(async () => {
            render(
              <StellarWalletProvider>
                <StateCapture />
              </StellarWalletProvider>
            )
          })

          await waitFor(() => {
            expect(capturedState).not.toBeNull()
          })

          // The invariant: isConnected === (publicKey !== null)
          expect(capturedState!.isConnected).toBe(capturedState!.publicKey !== null)
        }
      ),
      { numRuns: 10 }
    )
  })

  /**
   * Property 4: After disconnect, localStorage is always clean
   */
  it('localStorage is always clean after disconnect regardless of initial state', async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.record({
          hasAddress: fc.boolean(),
          hasAccessToken: fc.boolean(),
          hasRefreshToken: fc.boolean(),
        }),
        async ({ hasAddress, hasAccessToken, hasRefreshToken }) => {
          localStorage.clear()
          if (hasAddress) localStorage.setItem('wallet_address', MOCK_KEY)
          if (hasAccessToken) localStorage.setItem('access_token', 'token')
          if (hasRefreshToken) localStorage.setItem('refresh_token', 'refresh')

          await act(async () => {
            render(
              <StellarWalletProvider>
                <WalletConsumer />
              </StellarWalletProvider>
            )
          })

          await act(async () => {
            await userEvent.click(screen.getByText('Disconnect'))
          })

          expect(localStorage.getItem('access_token')).toBeNull()
          expect(localStorage.getItem('refresh_token')).toBeNull()
          expect(localStorage.getItem('wallet_address')).toBeNull()
        }
      ),
      { numRuns: 8 }
    )
  })
})
