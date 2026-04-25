import { useEffect, useState } from 'react';
import { ExternalLink, Loader2, CheckCircle2, XCircle, AlertCircle } from 'lucide-react';

export type TransactionStatus =
  | 'idle'
  | 'pending'
  | 'confirming'
  | 'confirmed'
  | 'error'
  | 'failed'   // alias for 'error', kept for backward compatibility
  | 'timeout';

export interface TransactionModalProps {
  isOpen: boolean;
  /** Transaction status. 'error' and 'failed' are equivalent. */
  status: TransactionStatus;
  txHash?: string;
  /** Error message to display. 'errorMessage' and 'error' are equivalent. */
  errorMessage?: string;
  /** @deprecated Use errorMessage instead */
  error?: string;
  onClose: () => void;
  onRetry?: () => void;
}

/**
 * TransactionModal - Displays transaction confirmation status with loading states
 *
 * Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.11
 *
 * Features:
 * - Loading states: Pending (waiting for wallet signature), Confirming (submitted, waiting for blockchain), Confirmed (success)
 * - Transaction status messages appropriate to each state
 * - Success toast with transaction hash link to Stellar explorer
 * - Error toast with failure reason and retry button
 * - 60-second timeout - if transaction not confirmed within 60s, shows timeout error
 */
export function TransactionModal({
  isOpen,
  status,
  txHash,
  errorMessage,
  error,
  onClose,
  onRetry,
}: TransactionModalProps) {
  const [timeoutSeconds, setTimeoutSeconds] = useState(60);

  // Normalize status: treat 'failed' as 'error' for display
  const normalizedStatus: TransactionStatus =
    status === 'failed' ? 'error' : status;

  // Resolve error message from either prop
  const resolvedError = errorMessage ?? error;

  // Timeout countdown while transaction is in-flight
  useEffect(() => {
    if (normalizedStatus === 'pending' || normalizedStatus === 'confirming') {
      setTimeoutSeconds(60);
      const interval = setInterval(() => {
        setTimeoutSeconds((prev) => {
          if (prev <= 1) {
            clearInterval(interval);
            return 0;
          }
          return prev - 1;
        });
      }, 1000);
      return () => clearInterval(interval);
    }
  }, [normalizedStatus]);

  if (!isOpen) return null;

  const getStatusContent = () => {
    switch (normalizedStatus) {
      case 'pending':
        return {
          icon: <Loader2 className="w-16 h-16 text-blue-600 animate-spin" />,
          title: 'Waiting for Signature',
          message: 'Waiting for wallet signature...',
          showClose: false,
          bgColor: 'bg-blue-50',
        };

      case 'confirming':
        return {
          icon: <Loader2 className="w-16 h-16 text-blue-600 animate-spin" />,
          title: 'Confirming on Stellar',
          message: 'Confirming on Stellar...',
          showClose: false,
          bgColor: 'bg-blue-50',
        };

      case 'confirmed':
        return {
          icon: <CheckCircle2 className="w-16 h-16 text-green-600" />,
          title: 'Transaction Confirmed',
          message: 'Your transaction has been successfully confirmed!',
          showClose: true,
          bgColor: 'bg-green-50',
        };

      case 'error':
        return {
          icon: <XCircle className="w-16 h-16 text-red-600" />,
          title: 'Transaction Failed',
          message: resolvedError || 'Your transaction could not be completed.',
          showClose: true,
          bgColor: 'bg-red-50',
        };

      case 'timeout':
        return {
          icon: <AlertCircle className="w-16 h-16 text-yellow-600" />,
          title: 'Transaction Timeout',
          message:
            'The transaction was not confirmed within 60 seconds. Please try again.',
          showClose: true,
          bgColor: 'bg-yellow-50',
        };

      default:
        return null;
    }
  };

  const content = getStatusContent();
  if (!content) return null;

  // Stellar explorer URL: https://stellar.expert/explorer/testnet/tx/{txHash}
  const network = import.meta.env.VITE_STELLAR_NETWORK === 'mainnet' ? 'mainnet' : 'testnet';
  const explorerUrl = txHash
    ? `https://stellar.expert/explorer/${network}/tx/${txHash}`
    : null;

  const isInFlight = normalizedStatus === 'pending' || normalizedStatus === 'confirming';
  const isErrorState = normalizedStatus === 'error' || normalizedStatus === 'timeout';

  return (
    <div
      className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
      role="dialog"
      aria-modal="true"
      aria-labelledby="transaction-modal-title"
    >
      <div className="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 overflow-hidden">
        {/* Header with status colour */}
        <div className={`${content.bgColor} p-6 flex flex-col items-center`}>
          {content.icon}
          <h2
            id="transaction-modal-title"
            className="text-2xl font-bold text-gray-900 mt-4 text-center"
          >
            {content.title}
          </h2>
        </div>

        {/* Body */}
        <div className="p-6">
          <p className="text-gray-600 text-center mb-4">{content.message}</p>

          {/* Countdown while in-flight */}
          {isInFlight && (
            <div className="text-center text-sm text-gray-500 mb-4">
              Timeout in {timeoutSeconds}s
            </div>
          )}

          {/* Transaction hash link on success */}
          {normalizedStatus === 'confirmed' && explorerUrl && (
            <a
              href={explorerUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center justify-center gap-2 text-blue-600 hover:text-blue-700 text-sm mb-4"
            >
              <span>View on Stellar Explorer</span>
              <ExternalLink className="w-4 h-4" />
            </a>
          )}

          {/* Action buttons */}
          <div className="flex gap-3">
            {/* Retry button for error / timeout */}
            {isErrorState && onRetry && (
              <button
                onClick={onRetry}
                className="flex-1 bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 transition-colors font-medium"
              >
                Retry
              </button>
            )}

            {/* Close button */}
            {content.showClose && (
              <button
                onClick={onClose}
                className={`${
                  isErrorState ? 'flex-1' : 'w-full'
                } bg-gray-200 text-gray-700 py-2 px-4 rounded-md hover:bg-gray-300 transition-colors font-medium`}
              >
                Close
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
