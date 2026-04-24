/**
 * useTransaction Hook - Manages transaction state and UI
 * 
 * Requirements: 9.1-9.11
 * 
 * Provides:
 * - Transaction status management
 * - Modal state management
 * - Error handling with toast notifications
 * - Success handling with transaction hash
 * - Retry functionality
 */

import { useState, useCallback } from 'react';
import {
  TransactionStatus,
  TransactionError,
  handleTransaction,
  showTransactionError,
  showTransactionSuccess,
} from '../lib/transaction-handler';

export interface UseTransactionOptions {
  onSuccess?: (txHash: string) => void;
  onError?: (error: TransactionError) => void;
  showToasts?: boolean;
}

export interface UseTransactionReturn {
  status: TransactionStatus;
  isModalOpen: boolean;
  txHash?: string;
  error?: TransactionError;
  execute: (transactionFn: () => Promise<{ hash: string }>) => Promise<void>;
  retry: () => Promise<void>;
  closeModal: () => void;
  reset: () => void;
}

/**
 * Hook for managing transaction execution with modal and error handling
 */
export function useTransaction(options: UseTransactionOptions = {}): UseTransactionReturn {
  const { onSuccess, onError, showToasts = true } = options;

  const [status, setStatus] = useState<TransactionStatus>('idle');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [txHash, setTxHash] = useState<string | undefined>();
  const [error, setError] = useState<TransactionError | undefined>();
  const [lastTransactionFn, setLastTransactionFn] = useState<
    (() => Promise<{ hash: string }>) | null
  >(null);

  const reset = useCallback(() => {
    setStatus('idle');
    setIsModalOpen(false);
    setTxHash(undefined);
    setError(undefined);
    setLastTransactionFn(null);
  }, []);

  const closeModal = useCallback(() => {
    setIsModalOpen(false);
    // Reset after a delay to allow modal close animation
    setTimeout(reset, 300);
  }, [reset]);

  const execute = useCallback(
    async (transactionFn: () => Promise<{ hash: string }>) => {
      // Store transaction function for retry
      setLastTransactionFn(() => transactionFn);

      // Reset state
      setError(undefined);
      setTxHash(undefined);

      // Open modal
      setIsModalOpen(true);

      // Execute transaction
      const result = await handleTransaction(transactionFn, {
        onStatusChange: setStatus,
        onSuccess: (hash) => {
          setTxHash(hash);
          if (showToasts) {
            showTransactionSuccess(hash);
          }
          if (onSuccess) {
            onSuccess(hash);
          }
        },
        onError: (err) => {
          setError(err);
          if (showToasts) {
            showTransactionError(err, () => retry());
          }
          if (onError) {
            onError(err);
          }
        },
      });

      return result;
    },
    [onSuccess, onError, showToasts]
  );

  const retry = useCallback(async () => {
    if (lastTransactionFn) {
      await execute(lastTransactionFn);
    }
  }, [lastTransactionFn, execute]);

  return {
    status,
    isModalOpen,
    txHash,
    error,
    execute,
    retry,
    closeModal,
    reset,
  };
}
