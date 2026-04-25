/**
 * Transaction Handler - Comprehensive transaction management with error handling
 * 
 * Requirements: 9.1-9.11
 * 
 * Features:
 * - Transaction status tracking
 * - Error classification and user-friendly messages
 * - Timeout handling (60 seconds)
 * - Retry logic for recoverable errors
 */

import React from 'react';
import { toast } from '../hooks/use-toast';
import { ToastAction } from '../components/ui/toast';

export type TransactionStatus = 'idle' | 'pending' | 'confirming' | 'confirmed' | 'failed' | 'timeout';

export interface TransactionError {
  type: 'insufficient_balance' | 'user_rejected' | 'network_error' | 'timeout' | 'unknown';
  message: string;
  isRecoverable: boolean;
}

export interface TransactionResult {
  success: boolean;
  txHash?: string;
  error?: TransactionError;
}

/**
 * Classify error type from error object
 */
export function classifyError(error: any): TransactionError {
  const errorMessage = error?.message?.toLowerCase() || '';
  const errorCode = error?.code?.toLowerCase() || '';

  // User rejected transaction
  if (
    errorMessage.includes('user rejected') ||
    errorMessage.includes('user denied') ||
    errorMessage.includes('cancelled') ||
    errorMessage.includes('user declined') ||
    errorCode === 'user_rejected'
  ) {
    return {
      type: 'user_rejected',
      message: 'Transaction cancelled by user.',
      isRecoverable: true,
    };
  }

  // Insufficient balance
  if (
    errorMessage.includes('insufficient') ||
    errorMessage.includes('not enough') ||
    errorMessage.includes('balance') ||
    errorMessage.includes('underfunded') ||
    errorMessage.includes('op_underfunded')
  ) {
    return {
      type: 'insufficient_balance',
      message: 'Insufficient XLM balance. Please add funds to your wallet and try again.',
      isRecoverable: false,
    };
  }

  // Network errors
  if (
    errorMessage.includes('network') ||
    errorMessage.includes('connection') ||
    errorMessage.includes('failed to fetch') ||
    errorMessage.includes('horizon') ||
    errorCode === 'econnrefused' ||
    errorCode === 'err_network'
  ) {
    return {
      type: 'network_error',
      message: 'Network error. Please check your connection and try again.',
      isRecoverable: true,
    };
  }

  // Timeout
  if (errorMessage.includes('timeout') || errorMessage.includes('timed out')) {
    return {
      type: 'timeout',
      message: 'Transaction timed out after 60 seconds. Please try again.',
      isRecoverable: true,
    };
  }

  // Unknown error
  return {
    type: 'unknown',
    message: error?.message || 'An unexpected error occurred. Please try again.',
    isRecoverable: true,
  };
}

/**
 * Display error toast with appropriate actions
 */
export function showTransactionError(error: TransactionError, onRetry?: () => void): void {
  const toastConfig: any = {
    variant: 'destructive',
    title: getErrorTitle(error.type),
    description: error.message,
  };

  // Add retry button for recoverable errors
  if (error.isRecoverable && onRetry) {
    toastConfig.action = (
      <ToastAction altText="Retry" onClick={onRetry}>
        Retry
      </ToastAction>
    );
  }

  // Add fund wallet button for insufficient balance
  if (error.type === 'insufficient_balance') {
    toastConfig.action = (
      <ToastAction
        altText="Fund Wallet"
        onClick={() => {
          window.location.href = '/wallet';
        }}
      >
        Fund Wallet
      </ToastAction>
    );
  }

  toast(toastConfig);
}

/**
 * Display success toast with transaction hash link
 */
export function showTransactionSuccess(txHash: string, message: string = 'Transaction confirmed!'): void {
  const explorerUrl = `https://${
    import.meta.env.VITE_STELLAR_NETWORK === 'mainnet' ? 'stellar' : 'testnet.steexp'
  }.expert/tx/${txHash}`;

  toast({
    variant: 'success',
    title: 'Success',
    description: message,
    action: (
      <ToastAction
        altText="View Transaction"
        onClick={() => {
          window.open(explorerUrl, '_blank', 'noopener,noreferrer');
        }}
      >
        View
      </ToastAction>
    ),
  });
}

/**
 * Get user-friendly error title based on error type
 */
function getErrorTitle(errorType: TransactionError['type']): string {
  switch (errorType) {
    case 'insufficient_balance':
      return 'Insufficient Balance';
    case 'user_rejected':
      return 'Transaction Cancelled';
    case 'network_error':
      return 'Network Error';
    case 'timeout':
      return 'Transaction Timeout';
    case 'unknown':
    default:
      return 'Transaction Failed';
  }
}

/**
 * Execute transaction with timeout and error handling
 */
export async function executeWithTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number = 60000
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      setTimeout(() => reject(new Error('Transaction timed out after 60 seconds')), timeoutMs)
    ),
  ]);
}

/**
 * Handle transaction lifecycle with status updates
 */
export interface TransactionHandlerCallbacks {
  onStatusChange: (status: TransactionStatus) => void;
  onSuccess: (txHash: string) => void;
  onError: (error: TransactionError) => void;
}

export async function handleTransaction(
  transactionFn: () => Promise<{ hash: string }>,
  callbacks: TransactionHandlerCallbacks
): Promise<TransactionResult> {
  const { onStatusChange, onSuccess, onError } = callbacks;

  try {
    // Pending: Waiting for user signature
    onStatusChange('pending');

    // Execute transaction with timeout
    const result = await executeWithTimeout(transactionFn(), 60000);

    // Confirming: Transaction submitted to network
    onStatusChange('confirming');

    // Confirmed: Transaction successful
    onStatusChange('confirmed');
    onSuccess(result.hash);

    return {
      success: true,
      txHash: result.hash,
    };
  } catch (error: any) {
    // Classify and handle error
    const txError = classifyError(error);
    
    // Set appropriate status
    if (txError.type === 'timeout') {
      onStatusChange('timeout');
    } else {
      onStatusChange('failed');
    }

    onError(txError);

    return {
      success: false,
      error: txError,
    };
  }
}
