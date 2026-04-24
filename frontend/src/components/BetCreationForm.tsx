import { useState } from 'react';
import { EncryptionService } from '../services/encryption';
import { useStellarWallet } from '../context/StellarWalletContext';
import { stellar } from '../lib/stellar-helper';
import rustApiClient from '../config/api';
import { handleError, handleSuccess } from '../lib/error-handler';
import { TransactionModal } from './TransactionModal';
import { useTransaction } from '../hooks/useTransaction';

interface BetCreationFormProps {
  onSuccess: (betId: string, shareableUrl: string) => void;
  onCancel: () => void;
}

interface BetFormData {
  question: string;
  stakeAmount: string;
  endTime: string;
}

export function BetCreationForm({ onSuccess, onCancel }: BetCreationFormProps) {
  const { publicKey } = useStellarWallet();
  const transaction = useTransaction({
    onSuccess: (txHash) => {
      // Transaction confirmed, now create backend record
      createBackendRecord(txHash);
    },
    showToasts: true,
  });
  
  const [formData, setFormData] = useState<BetFormData>({
    question: '',
    stakeAmount: '',
    endTime: '',
  });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [showSuccessModal, setShowSuccessModal] = useState(false);
  const [successData, setSuccessData] = useState<{ betId: string; shareableUrl: string } | null>(null);
  const [pendingTxData, setPendingTxData] = useState<{
    stakeInStroops: number;
    endTimeTimestamp: number;
    urlHash: string;
  } | null>(null);

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    // Question validation
    if (!formData.question.trim()) {
      newErrors.question = 'Question is required';
    } else if (formData.question.length < 10) {
      newErrors.question = 'Question must be at least 10 characters';
    } else if (formData.question.length > 200) {
      newErrors.question = 'Question must be at most 200 characters';
    } else if (!formData.question.includes('?')) {
      newErrors.question = 'Question must include a question mark';
    }

    // Stake amount validation
    const stake = parseFloat(formData.stakeAmount);
    if (!formData.stakeAmount) {
      newErrors.stakeAmount = 'Stake amount is required';
    } else if (isNaN(stake) || stake <= 0) {
      newErrors.stakeAmount = 'Stake amount must be positive';
    }

    // End time validation
    if (!formData.endTime) {
      newErrors.endTime = 'End time is required';
    } else {
      const endTime = new Date(formData.endTime);
      const now = new Date();
      if (endTime <= now) {
        newErrors.endTime = 'End time must be in the future';
      }
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const createBackendRecord = async (txHash: string) => {
    if (!pendingTxData) return;

    try {
      // Call backend API to create bet record
      const response = await rustApiClient.post('/api/v1/p2p-bets', {
        question: formData.question,
        stake_amount: pendingTxData.stakeInStroops,
        end_time: formData.endTime,
        shareable_url_hash: pendingTxData.urlHash,
        transaction_hash: txHash,
      });

      const { bet_id, shareable_url } = response.data;

      // Show success modal
      setSuccessData({ betId: bet_id, shareableUrl: shareable_url });
      setShowSuccessModal(true);
      
      handleSuccess('Bet Created', 'Your bet has been created successfully!');
    } catch (error: any) {
      console.error('Error creating bet record:', error);
      handleError(error, {
        title: 'Failed to Create Bet Record',
      });
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    if (!publicKey) {
      handleError(new Error('Wallet not connected'), {
        title: 'Wallet Required',
      });
      return;
    }

    try {
      // Convert stake to stroops (1 XLM = 10,000,000 stroops)
      const stakeInStroops = Math.floor(parseFloat(formData.stakeAmount) * 10_000_000);
      const endTimeTimestamp = Math.floor(new Date(formData.endTime).getTime() / 1000);

      // Generate shareable URL hash
      const urlHash = await EncryptionService.encryptBetId(
        `temp-${Date.now()}`,
        import.meta.env.VITE_ENCRYPTION_SECRET || 'default_secret'
      );

      // Store data for backend record creation after transaction confirms
      setPendingTxData({ stakeInStroops, endTimeTimestamp, urlHash });

      // Execute transaction
      await transaction.execute(async () => {
        const contractId = import.meta.env.VITE_STELLAR_P2P_BET_CONTRACT_ID;
        
        if (!contractId) {
          throw new Error('P2P Bet contract ID not configured');
        }

        // Build the contract call transaction
        const StellarSdk = await import('@stellar/stellar-sdk');
        const server = new StellarSdk.Horizon.Server(
          import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
            ? 'https://horizon.stellar.org'
            : 'https://horizon-testnet.stellar.org'
        );

        const account = await server.loadAccount(publicKey);
        const contract = new StellarSdk.Contract(contractId);

        // Build create_bet operation
        const txn = new StellarSdk.TransactionBuilder(account, {
          fee: StellarSdk.BASE_FEE,
          networkPassphrase: import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
            ? StellarSdk.Networks.PUBLIC
            : StellarSdk.Networks.TESTNET,
        })
          .addOperation(
            contract.call(
              'create_bet',
              StellarSdk.Address.fromString(publicKey).toScVal(),
              StellarSdk.nativeToScVal(formData.question, { type: 'string' }),
              StellarSdk.nativeToScVal(stakeInStroops, { type: 'i128' }),
              StellarSdk.nativeToScVal(endTimeTimestamp, { type: 'u64' }),
              StellarSdk.nativeToScVal(urlHash, { type: 'string' })
            )
          )
          .setTimeout(180)
          .build();

        // Sign transaction with Freighter
        const { signTransaction } = await import('@stellar/freighter-api');
        const { signedTxXdr, error: signError } = await signTransaction(txn.toXDR(), {
          networkPassphrase: import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
            ? StellarSdk.Networks.PUBLIC
            : StellarSdk.Networks.TESTNET,
        });

        if (signError) {
          throw new Error(`Transaction signing failed: ${signError}`);
        }

        // Submit transaction to Stellar
        const signedTransaction = StellarSdk.TransactionBuilder.fromXDR(
          signedTxXdr,
          import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
            ? StellarSdk.Networks.PUBLIC
            : StellarSdk.Networks.TESTNET
        );

        const result = await server.submitTransaction(signedTransaction as any);

        if (!result.successful) {
          throw new Error('Transaction failed on Stellar network');
        }

        return { hash: result.hash };
      });
    } catch (error: any) {
      console.error('Error creating bet:', error);
      handleError(error, {
        title: 'Bet Creation Failed',
        onRetry: () => handleSubmit(e),
      });
    }
  };

  const handleCopyLink = async () => {
    if (!successData) return;

    try {
      await navigator.clipboard.writeText(successData.shareableUrl);
      handleSuccess('Link Copied', 'Shareable link copied to clipboard!');
    } catch (error) {
      handleError(error, { title: 'Copy Failed' });
    }
  };

  const handleSuccessClose = () => {
    setShowSuccessModal(false);
    if (successData) {
      onSuccess(successData.betId, successData.shareableUrl);
    }
  };

  return (
    <>
      {/* Transaction Modal */}
      <TransactionModal
        isOpen={transaction.isModalOpen}
        status={transaction.status}
        txHash={transaction.txHash}
        error={transaction.error?.message}
        onClose={transaction.closeModal}
        onRetry={transaction.retry}
      />

      {/* Success Modal */}
      {showSuccessModal && successData && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 max-w-md w-full mx-4">
            <div className="text-center mb-6">
              <div className="mx-auto w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mb-4">
                <svg
                  className="w-8 h-8 text-green-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              </div>
              <h2 className="text-2xl font-bold text-gray-900 mb-2">Bet Created!</h2>
              <p className="text-gray-600">Your bet has been created successfully on the Stellar blockchain.</p>
            </div>

            <div className="bg-gray-50 rounded-lg p-4 mb-6">
              <p className="text-sm text-gray-600 mb-2">Shareable Link:</p>
              <p className="text-sm font-mono text-gray-800 break-all">{successData.shareableUrl}</p>
            </div>

            <div className="flex gap-3">
              <button
                onClick={handleCopyLink}
                className="flex-1 bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 transition-colors"
              >
                Copy Link
              </button>
              <button
                onClick={handleSuccessClose}
                className="flex-1 bg-gray-200 text-gray-700 py-2 px-4 rounded-md hover:bg-gray-300 transition-colors"
              >
                Done
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Form */}
      <div className="bg-white rounded-lg shadow-lg p-6 max-w-2xl mx-auto">
        <h2 className="text-2xl font-bold mb-6">Create Your Bet</h2>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="question" className="block text-sm font-medium text-gray-700 mb-1">
              Question
            </label>
            <textarea
              id="question"
              value={formData.question}
              onChange={(e) => setFormData({ ...formData, question: e.target.value })}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
              placeholder="Will it rain tomorrow?"
              rows={3}
              maxLength={200}
            />
            <div className="flex justify-between items-center mt-1">
              <div>
                {errors.question && <p className="text-red-500 text-sm">{errors.question}</p>}
              </div>
              <p className="text-xs text-gray-500">{formData.question.length}/200</p>
            </div>
          </div>

          <div>
            <label htmlFor="stakeAmount" className="block text-sm font-medium text-gray-700 mb-1">
              Stake Amount (XLM)
            </label>
            <input
              type="number"
              id="stakeAmount"
              value={formData.stakeAmount}
              onChange={(e) => setFormData({ ...formData, stakeAmount: e.target.value })}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="10"
              step="0.01"
              min="0"
            />
            {errors.stakeAmount && <p className="text-red-500 text-sm mt-1">{errors.stakeAmount}</p>}
          </div>

          <div>
            <label htmlFor="endTime" className="block text-sm font-medium text-gray-700 mb-1">
              End Time
            </label>
            <input
              type="datetime-local"
              id="endTime"
              value={formData.endTime}
              onChange={(e) => setFormData({ ...formData, endTime: e.target.value })}
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            {errors.endTime && <p className="text-red-500 text-sm mt-1">{errors.endTime}</p>}
          </div>

          <div className="flex gap-4 pt-4">
            <button
              type="submit"
              disabled={transaction.status === 'pending' || transaction.status === 'confirming'}
              className="flex-1 bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
            >
              {transaction.status === 'pending' || transaction.status === 'confirming' ? 'Creating...' : 'Create Bet'}
            </button>
            <button
              type="button"
              onClick={onCancel}
              disabled={transaction.status === 'pending' || transaction.status === 'confirming'}
              className="flex-1 bg-gray-200 text-gray-700 py-2 px-4 rounded-md hover:bg-gray-300 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
            >
              Cancel
            </button>
          </div>
        </form>
      </div>
    </>
  );
}
