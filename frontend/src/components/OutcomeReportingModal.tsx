import { useState } from 'react';
import { Bet } from '../types/p2p-bet';
import { useStellarWallet } from '../context/StellarWalletContext';
import { handleError, handleSuccess } from '../lib/error-handler';
import rustApiClient from '../config/api';
import { TransactionModal } from './TransactionModal';
import { useTransaction } from '../hooks/useTransaction';

interface OutcomeReportingModalProps {
  betId: string;
  bet: Bet;
  userAddress: string;
  onClose: () => void;
}

export function OutcomeReportingModal({
  betId,
  bet,
  userAddress,
  onClose,
}: OutcomeReportingModalProps) {
  const { publicKey } = useStellarWallet();
  const transaction = useTransaction({
    onSuccess: (txHash) => {
      // Transaction confirmed, now record in backend
      recordOutcomeInBackend(txHash);
    },
    showToasts: true,
  });
  
  const [selectedOutcome, setSelectedOutcome] = useState<boolean | null>(null);
  const [error, setError] = useState<string>('');

  const userParticipant = bet.participants.find((p) => p.address === userAddress);
  const hasReported = userParticipant?.hasReported || false;
  const isFirstReporter = bet.outcomeReports.length === 0;

  const recordOutcomeInBackend = async (txHash: string) => {
    if (selectedOutcome === null) return;

    try {
      const endpoint = isFirstReporter ? 'report-outcome' : 'confirm-outcome';
      
      // Call backend API to record outcome
      await rustApiClient.post(`/api/v1/p2p-bets/${betId}/${endpoint}`, {
        outcome: selectedOutcome,
        transaction_hash: txHash,
      });

      handleSuccess(
        isFirstReporter ? 'Outcome Reported' : 'Outcome Confirmed',
        isFirstReporter 
          ? 'Your outcome report has been recorded on-chain.'
          : 'Your outcome confirmation has been recorded on-chain.'
      );

      onClose();
    } catch (err: any) {
      console.error('Error recording outcome:', err);
      handleError(err, {
        title: 'Failed to Record Outcome',
      });
    }
  };

  const handleSubmit = async () => {
    if (selectedOutcome === null) {
      setError('Please select an outcome');
      return;
    }

    if (!publicKey) {
      handleError(new Error('Wallet not connected'), {
        title: 'Wallet Required',
      });
      return;
    }

    setError('');

    try {
      const contractFunction = isFirstReporter ? 'report_outcome' : 'confirm_outcome';

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

        // Build report_outcome or confirm_outcome operation
        const txn = new StellarSdk.TransactionBuilder(account, {
          fee: StellarSdk.BASE_FEE,
          networkPassphrase: import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
            ? StellarSdk.Networks.PUBLIC
            : StellarSdk.Networks.TESTNET,
        })
          .addOperation(
            contract.call(
              contractFunction,
              StellarSdk.Address.fromString(publicKey).toScVal(),
              StellarSdk.nativeToScVal(parseInt(betId), { type: 'u64' }),
              StellarSdk.nativeToScVal(selectedOutcome, { type: 'bool' })
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
    } catch (err: any) {
      console.error('Error submitting outcome:', err);
      handleError(err, {
        title: 'Outcome Submission Failed',
        onRetry: handleSubmit,
      });
      setError('Failed to submit outcome. Please try again.');
    }
  };

  if (hasReported) {
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
        
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 max-w-md w-full">
            <h2 className="text-xl font-bold mb-4">Outcome Status</h2>
            <p className="text-gray-700 mb-4">You have already reported the outcome.</p>
            {bet.state === 'Disputed' && (
              <div className="bg-yellow-100 border border-yellow-400 text-yellow-700 px-4 py-3 rounded mb-4">
                <p className="font-bold">Disputed</p>
                <p className="text-sm">
                  Participants disagree on the outcome. Manual resolution required.
                </p>
              </div>
            )}
            {bet.state === 'Verified' && (
              <div className="bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded mb-4">
                <p className="font-bold">Verified</p>
                <p className="text-sm">
                  Outcome: {bet.verifiedOutcome ? 'Yes' : 'No'}
                </p>
              </div>
            )}
            <button
              onClick={onClose}
              className="w-full bg-gray-200 text-gray-700 py-2 px-4 rounded-md hover:bg-gray-300"
            >
              Close
            </button>
          </div>
        </div>
      </>
    );
  }

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
      
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg p-6 max-w-md w-full">
          <h2 className="text-xl font-bold mb-4">
            {isFirstReporter ? 'Report Outcome' : 'Confirm Outcome'}
          </h2>

          <p className="text-gray-700 mb-4">{bet.question}</p>

          {!isFirstReporter && bet.outcomeReports.length > 0 && (
            <div className="bg-blue-50 border border-blue-200 p-3 rounded mb-4">
              <p className="text-sm text-gray-700">
                First report: <span className="font-bold">{bet.outcomeReports[0].outcome ? 'Yes' : 'No'}</span>
              </p>
            </div>
          )}

          <div className="space-y-3 mb-6">
            <button
              onClick={() => setSelectedOutcome(true)}
              className={`w-full py-3 px-4 rounded-md border-2 transition-colors ${
                selectedOutcome === true
                  ? 'border-green-500 bg-green-50 text-green-700'
                  : 'border-gray-300 hover:border-green-300'
              }`}
            >
              Yes
            </button>
            <button
              onClick={() => setSelectedOutcome(false)}
              className={`w-full py-3 px-4 rounded-md border-2 transition-colors ${
                selectedOutcome === false
                  ? 'border-red-500 bg-red-50 text-red-700'
                  : 'border-gray-300 hover:border-red-300'
              }`}
            >
              No
            </button>
          </div>

          {error && <p className="text-red-500 text-sm mb-4">{error}</p>}

          <div className="flex gap-4">
            <button
              onClick={handleSubmit}
              disabled={transaction.status === 'pending' || transaction.status === 'confirming' || selectedOutcome === null}
              className="flex-1 bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed"
            >
              {transaction.status === 'pending' || transaction.status === 'confirming' ? 'Submitting...' : 'Submit'}
            </button>
            <button
              onClick={onClose}
              disabled={transaction.status === 'pending' || transaction.status === 'confirming'}
              className="flex-1 bg-gray-200 text-gray-700 py-2 px-4 rounded-md hover:bg-gray-300 disabled:bg-gray-300 disabled:cursor-not-allowed"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
