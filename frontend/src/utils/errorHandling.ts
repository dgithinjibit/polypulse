/**
 * Error Handling Utilities for P2P Betting Components
 *
 * Requirements: 9.9, 9.10
 *
 * Provides user-friendly error messages for common Soroban/Freighter errors:
 * - Insufficient balance
 * - User rejection / cancelled signature
 * - Network errors
 * - Contract errors
 */

/**
 * Parse a Soroban/Freighter/network error into a user-friendly message.
 *
 * @param error - The raw error thrown during a transaction or API call
 * @param requiredAmount - Optional XLM amount required (used in insufficient balance message)
 * @returns A user-friendly error string
 */
export function parseTransactionError(error: unknown, requiredAmount?: number): string {
  const message = extractMessage(error);
  const lower = message.toLowerCase();

  // --- User rejection / cancelled signature (Requirement 9.10) ---
  if (
    lower.includes('user declined') ||
    lower.includes('user rejected') ||
    lower.includes('user denied') ||
    lower.includes('rejected') ||
    lower.includes('cancelled') ||
    lower.includes('canceled') ||
    lower.includes('user cancel')
  ) {
    return 'Transaction cancelled by user.';
  }

  // --- Insufficient balance (Requirement 9.9) ---
  if (
    lower.includes('insufficient balance') ||
    lower.includes('insufficient_funds') ||
    lower.includes('insufficient funds') ||
    lower.includes('not enough') ||
    lower.includes('underfunded') ||
    lower.includes('op_underfunded')
  ) {
    if (requiredAmount !== undefined && requiredAmount > 0) {
      return `Insufficient XLM balance. You need at least ${requiredAmount.toFixed(2)} XLM to place this bet.`;
    }
    return 'Insufficient XLM balance. Please add funds to your wallet and try again.';
  }

  // --- Network / connection errors ---
  if (
    lower.includes('network') ||
    lower.includes('connection') ||
    lower.includes('econnrefused') ||
    lower.includes('err_network') ||
    lower.includes('failed to fetch') ||
    lower.includes('net::err') ||
    lower.includes('horizon')
  ) {
    return 'Network error. Please check your connection and try again.';
  }

  // --- Transaction timeout ---
  if (lower.includes('timeout') || lower.includes('timed out')) {
    return 'Transaction timed out. Please try again.';
  }

  // --- Contract / Soroban specific errors ---
  if (lower.includes('contract') || lower.includes('soroban') || lower.includes('invoke')) {
    return 'Smart contract error. Please try again or contact support if the issue persists.';
  }

  // --- Wallet not connected ---
  if (lower.includes('wallet') && (lower.includes('not connected') || lower.includes('not installed'))) {
    return 'Wallet not connected. Please connect your Freighter wallet and try again.';
  }

  // --- Fallback: return the raw message if it's short enough, otherwise generic ---
  if (message && message.length <= 120) {
    return message;
  }

  return 'An unexpected error occurred. Please try again.';
}

/**
 * Extract a string message from an unknown error value.
 */
function extractMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message;
  if (typeof error === 'object' && error !== null) {
    const obj = error as Record<string, unknown>;
    if (typeof obj['message'] === 'string') return obj['message'];
    if (typeof obj['error'] === 'string') return obj['error'];
    // Axios-style error
    if (obj['response'] && typeof (obj['response'] as any)?.data?.error === 'string') {
      return (obj['response'] as any).data.error as string;
    }
  }
  return '';
}
