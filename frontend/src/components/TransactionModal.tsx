import { useEffect, useState } from 'react';
import { ExternalLink, Loader2, CheckCircle2, XCircle, AlertCircle } from 'lucide-react';

export type TransactionStatus = 'idle' | 'pending' | 'confirming' | 'confirmed' | 'failed' | 'timeout';

export interface TransactionModalProps {
  isOpen: boolean;
  status: TransactionStatus;
  txHash?: string;
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
 * - Loading states (Pending, Confirming, Confirmed)
 * - Transaction status messages
 * - Success display with transaction hash link
 * - Error display with failure reason and retry button
 * - 60-second timeout handling
 */
export function TransactionModal({
  isOpen,
  status,
  txHash,
  error,
  onClose,
  onRetry,
}: TransactionModalProps) {
  const [timeoutSeconds, setTimeoutSeconds] = useState(60);

  // Timeout countdown
  useEffect(() => {
    if (status === 'pending' || status === 'confirming') {
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
    } else {
      setTimeoutSeconds(60);
    }
  }, [status]);

  // Auto-trigger timeout after 60 seconds
  useEffect(() => {
    if (timeoutSeconds === 0 && (status === 'pending' || status === 'confirming')) {
      // Timeout will be handled by parent component
    }
  }, [timeoutSeconds, status]);

  if (!isOpen) return null;

  const getStatusContent = () => {
    switch (status) {
      case 'pending':
        return {
          icon: <Loader2 className="w-16 h-16 text-blue-600 animate-spin" />,
          title: 'Waiting for Signature',
          message: 'Please sign the transaction in your wallet...',
          showClose: false,
          bgColor: 'bg-blue-50',
        };

      case 'confirming':
        return {
          icon: <Loader2 className="w-16 h-16 text-blue-600 animate-spin" />,
          title: 'Confirming on Stellar',
          message: 'Your transaction is being confirmed on the blockchain...',
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

      case 'failed':
        return {
          icon: <XCircle className="w-16 h-16 text-red-600" />,
          title: 'Transaction Failed',
          message: error || 'Your transaction could not be completed.',
          showClose: true,
          bgColor: 'bg-red-50',
        };

      case 'timeout':
        return {
          icon: <AlertCircle className="w-16 h-16 text-yellow-600" />,
          title: 'Transaction Timeout',
          message: 'The transaction took too long to complete. Please try again.',
          showClose: true,
          bgColor: 'bg-yellow-50',
        };

      default:
        return null;
    }
  };

  const content = getStatusContent();
  if (!content) return null;

  const explorerUrl = txHash
    ? `https://${import.meta.env.VITE_STELLAR_NETWORK === 'mainnet' ? 'stellar' : 'testnet.steexp'}.expert/tx/${txHash}`
    : null;

  return (
    <div
      className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
      role="dialog"
      aria-modal="true"
      aria-labelledby="transaction-modal-title"
    >
      <div className="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 overflow-hidden">
        {/* Header with status color */}
        <div className={`${content.bgColor} p-6 flex flex-col items-center`}>
          {content.icon}
          <h2
            id="transaction-modal-title"
            className="text-2xl font-bold text-gray-900 mt-4 text-center"
          >
            {content.title}
          </h2>
        </div>

        {/* Content */}
        <div className="p-6">
          <p className="text-gray-600 text-center mb-4">{content.message}</p>

          {/* Timeout countdown for pending/confirming states */}
          {(status === 'pending' || status === 'confirming') && (
            <div className="text-center text-sm text-gray-500 mb-4">
              Timeout in {timeoutSeconds}s
            </div>
          )}

          {/* Transaction hash link for confirmed transactions */}
          {status === 'confirmed' && explorerUrl && (
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
            {/* Retry button for failed/timeout states */}
            {(status === 'failed' || status === 'timeout') && onRetry && (
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
                  status === 'failed' || status === 'timeout' ? 'flex-1' : 'w-full'
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
