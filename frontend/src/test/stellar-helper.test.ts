/**
 * Unit tests for StellarHelper
 * Tests wallet connection, balance fetching, signing, and error handling.
 * All Freighter API calls and Horizon server calls are mocked.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import * as fc from 'fast-check'

// ── Mock @stellar/freighter-api before importing StellarHelper ──────────────
vi.mock('@stellar/freighter-api', () => ({
  requestAccess: vi.fn(),
  getAddress: vi.fn(),
  signTransaction: vi.fn(),
}))

// ── Mock @stellar/stellar-sdk (only the parts we use) ───────────────────────
vi.mock('@stellar/stellar-sdk', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@stellar/stellar-sdk')>()
  return {
    ...actual,
    Horizon: {
      Server: vi.fn().mockImplementation(() => ({
        loadAccount: vi.fn(),
        payments: vi.fn(),
        submitTransaction: vi.fn(),
      })),
    },
  }
})

import { requestAccess, getAddress, signTransaction as freighterSign } from '@stellar/freighter-api'
import * as StellarSdk from '@stellar/stellar-sdk'
import {
  StellarHelper,
  WalletConnectionError,
  WalletRejectedError,
  SignatureError,
} from '../lib/stellar-helper'

// A valid-looking Stellar public key for tests
const MOCK_PUBLIC_KEY = 'GBTEST1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCDE'

describe('StellarHelper', () => {
  let helper: StellarHelper
  let mockServer: any

  beforeEach(() => {
    vi.clearAllMocks()
    helper = new StellarHelper('testnet')
    // Grab the mock server instance created by the constructor
    mockServer = (StellarSdk.Horizon.Server as any).mock.results[
      (StellarSdk.Horizon.Server as any).mock.results.length - 1
    ].value
  })

  // ── connectWallet ──────────────────────────────────────────────────────────

  describe('connectWallet()', () => {
    it('returns public key on successful connection', async () => {
      ;(requestAccess as any).mockResolvedValue({ address: MOCK_PUBLIC_KEY, error: null })

      const result = await helper.connectWallet()

      expect(result).toBe(MOCK_PUBLIC_KEY)
    })

    it('throws WalletRejectedError when user declines', async () => {
      ;(requestAccess as any).mockResolvedValue({
        address: '',
        error: 'User declined access',
      })

      await expect(helper.connectWallet()).rejects.toThrow(WalletRejectedError)
    })

    it('throws WalletConnectionError on generic access error', async () => {
      ;(requestAccess as any).mockResolvedValue({
        address: '',
        error: 'Extension crashed',
      })

      await expect(helper.connectWallet()).rejects.toThrow(WalletConnectionError)
    })

    it('throws WalletConnectionError when address is empty with no error', async () => {
      ;(requestAccess as any).mockResolvedValue({ address: '', error: null })

      await expect(helper.connectWallet()).rejects.toThrow(WalletConnectionError)
    })
  })

  // ── getPublicKey ───────────────────────────────────────────────────────────

  describe('getPublicKey()', () => {
    it('returns null when getAddress returns error', async () => {
      ;(getAddress as any).mockResolvedValue({ address: '', error: 'not connected' })

      const result = await helper.getPublicKey()

      expect(result).toBeNull()
    })

    it('returns cached key after connectWallet()', async () => {
      ;(requestAccess as any).mockResolvedValue({ address: MOCK_PUBLIC_KEY, error: null })
      await helper.connectWallet()

      // getAddress should NOT be called — uses cache
      const result = await helper.getPublicKey()

      expect(result).toBe(MOCK_PUBLIC_KEY)
      expect(getAddress).not.toHaveBeenCalled()
    })

    it('returns null when getAddress throws', async () => {
      ;(getAddress as any).mockRejectedValue(new Error('extension unavailable'))

      const result = await helper.getPublicKey()

      expect(result).toBeNull()
    })
  })

  // ── getBalance ─────────────────────────────────────────────────────────────

  describe('getBalance()', () => {
    it('returns xlm balance and empty assets for native-only account', async () => {
      mockServer.loadAccount.mockResolvedValue({
        balances: [{ asset_type: 'native', balance: '100.0000000' }],
      })

      const result = await helper.getBalance(MOCK_PUBLIC_KEY)

      expect(result.xlm).toBe('100.0000000')
      expect(result.assets).toHaveLength(0)
    })

    it('returns xlm and asset balances for multi-asset account', async () => {
      mockServer.loadAccount.mockResolvedValue({
        balances: [
          { asset_type: 'native', balance: '50.0000000' },
          { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GISSUER', balance: '200.0000000' },
        ],
      })

      const result = await helper.getBalance(MOCK_PUBLIC_KEY)

      expect(result.xlm).toBe('50.0000000')
      expect(result.assets).toHaveLength(1)
      expect(result.assets[0]).toEqual({ code: 'USDC', issuer: 'GISSUER', balance: '200.0000000' })
    })

    it('returns xlm as "0" when no native balance found', async () => {
      mockServer.loadAccount.mockResolvedValue({
        balances: [
          { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GISSUER', balance: '10.0' },
        ],
      })

      const result = await helper.getBalance(MOCK_PUBLIC_KEY)

      expect(result.xlm).toBe('0')
    })
  })

  // ── signAuthMessage ────────────────────────────────────────────────────────

  describe('signAuthMessage()', () => {
    beforeEach(async () => {
      // Connect wallet first so publicKey is set
      ;(requestAccess as any).mockResolvedValue({ address: MOCK_PUBLIC_KEY, error: null })
      await helper.connectWallet()

      mockServer.loadAccount.mockResolvedValue({
        id: MOCK_PUBLIC_KEY,
        sequence: '1234',
        balances: [{ asset_type: 'native', balance: '100.0000000' }],
        incrementSequenceNumber: vi.fn(),
      })
    })

    it('returns signature and publicKey on success', async () => {
      ;(freighterSign as any).mockResolvedValue({ signedTxXdr: 'SIGNED_XDR_STRING', error: null })

      const result = await helper.signAuthMessage('PolyPulse Login\nAddress: GBTEST\nNonce: abc123')

      expect(result.signature).toBe('SIGNED_XDR_STRING')
      expect(result.publicKey).toBe(MOCK_PUBLIC_KEY)
    })

    it('throws SignatureError when freighter returns error', async () => {
      ;(freighterSign as any).mockResolvedValue({ signedTxXdr: '', error: 'User rejected' })

      await expect(
        helper.signAuthMessage('test message')
      ).rejects.toThrow(SignatureError)
    })

    it('throws SignatureError when signedTxXdr is empty with no error', async () => {
      ;(freighterSign as any).mockResolvedValue({ signedTxXdr: '', error: null })

      await expect(
        helper.signAuthMessage('test message')
      ).rejects.toThrow(SignatureError)
    })

    it('throws Error when no wallet is connected', async () => {
      await helper.disconnect()

      await expect(
        helper.signAuthMessage('test message')
      ).rejects.toThrow('No wallet connected')
    })
  })

  // ── disconnect ─────────────────────────────────────────────────────────────

  describe('disconnect()', () => {
    it('returns true and clears cached public key', async () => {
      ;(requestAccess as any).mockResolvedValue({ address: MOCK_PUBLIC_KEY, error: null })
      await helper.connectWallet()

      const result = await helper.disconnect()

      expect(result).toBe(true)
      // After disconnect, getPublicKey should return null (no cache)
      ;(getAddress as any).mockResolvedValue({ address: '', error: 'not connected' })
      expect(await helper.getPublicKey()).toBeNull()
    })

    it('is idempotent — calling disconnect twice returns true both times', async () => {
      const first = await helper.disconnect()
      const second = await helper.disconnect()

      expect(first).toBe(true)
      expect(second).toBe(true)
    })
  })

  // ── formatAddress ──────────────────────────────────────────────────────────

  describe('formatAddress()', () => {
    it('shortens a long address to start...end format', () => {
      const result = helper.formatAddress('GABCDEFGHIJKLMNOPQRSTUVWXYZ')
      expect(result).toBe('GABC...WXYZ')
    })

    it('returns address as-is when shorter than startChars + endChars', () => {
      expect(helper.formatAddress('GABC')).toBe('GABC')
    })
  })

  // ── isFreighterInstalled ───────────────────────────────────────────────────

  describe('isFreighterInstalled()', () => {
    it('returns false when window.freighter is not present', () => {
      const original = (window as any).freighter
      delete (window as any).freighter

      expect(helper.isFreighterInstalled()).toBe(false)

      ;(window as any).freighter = original
    })

    it('returns true when window.freighter is present', () => {
      ;(window as any).freighter = {}

      expect(helper.isFreighterInstalled()).toBe(true)

      delete (window as any).freighter
    })
  })
})

// ── Property-Based Tests ───────────────────────────────────────────────────

describe('StellarHelper - Property-Based Tests', () => {
  let helper: StellarHelper

  beforeEach(() => {
    vi.clearAllMocks()
    helper = new StellarHelper('testnet')
  })

  it('formatAddress always produces "start...end" for addresses longer than 8 chars', () => {
    fc.assert(
      fc.property(
        fc.string({ minLength: 9, maxLength: 56 }),
        (addr) => {
          const result = helper.formatAddress(addr)
          expect(result).toContain('...')
          expect(result.startsWith(addr.slice(0, 4))).toBe(true)
          expect(result.endsWith(addr.slice(-4))).toBe(true)
        }
      )
    )
  })

  it('disconnect is always idempotent — always returns true', async () => {
    await fc.assert(
      fc.asyncProperty(fc.integer({ min: 1, max: 5 }), async (times) => {
        for (let i = 0; i < times; i++) {
          const result = await helper.disconnect()
          expect(result).toBe(true)
        }
      })
    )
  })

  it('getBalance xlm is always a non-negative numeric string', async () => {
    const mockServer = (StellarSdk.Horizon.Server as any).mock.results[
      (StellarSdk.Horizon.Server as any).mock.results.length - 1
    ].value

    await fc.assert(
      fc.asyncProperty(
        fc.float({ min: 0, max: 1_000_000, noNaN: true }),
        async (xlmAmount) => {
          const balanceStr = xlmAmount.toFixed(7)
          mockServer.loadAccount.mockResolvedValue({
            balances: [{ asset_type: 'native', balance: balanceStr }],
          })

          const result = await helper.getBalance(MOCK_PUBLIC_KEY)

          expect(parseFloat(result.xlm)).toBeGreaterThanOrEqual(0)
        }
      )
    )
  })
})
