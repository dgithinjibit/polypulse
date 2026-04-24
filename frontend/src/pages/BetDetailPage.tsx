import { useState, useEffect, useCallback } from 'react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { Bet, BetState, BetUpdate } from '../types/p2p-bet';
import { OutcomeReportingModal } from '../components/OutcomeReportingModal';
import { useStellarWallet } from '../context/StellarWalletContext';
import { useWebSocket } from '../context/WebSocketContext';
import { handleError, handleSuccess } from '../lib/error-handler';
import rustApiClient from '../config/api';
import LoadingOverlay from '../components/LoadingOverlay';
import { EncryptionService } from '../services/encryption';

export default function BetDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { publicKey } = useStellarWallet();
  const { subscribeToBet } = useWebSocket();

  const [bet, setBet] = useState<Bet | null>(null);
  const [loading, setLoading] = useState(true);
  const [showOutcomeModal, setShowOutcomeModal] = useState(false);
  const [joinLoading, setJoinLoading] = useState(false);
  const [joinPosition, setJoinPosition] = useState<'Yes' | 'No' | null>(null);
  const [stakeAmount, setStakeAmount] = useState('');
  const [decryptionError, setDecryptionError] = useState<string | null>(null);

  // Requirement 3.6: Decrypt bet ID from shareable URL
  const decryptBetId = useCallback(async (encryptedId: string): Promise<string | null> => {
    try {
      const secret = import.meta.env.VITE_ENCRYPTION_SECRET || 'default_secret';
      const decryptedId = await EncryptionService.decryptBetId(encryptedId, secret);
      return decryptedId;
    } catch (error) {
      console.error('Error decrypting bet ID:', error);
      // Return null to indicate decryption failure
      return null;
    }
  }, []);

  // Requirement 3.6: Determine if ID is encrypted and decrypt if needed
  const resolveBetId = useCallback(async (): Promise<string | null> => {
    if (!id) return null;

    // Check if there's a 'bet' query parameter (encrypted shareable URL format)
    const encryptedParam = searchParams.get('bet');
    if (encryptedParam) {
      // URL format: /bet/:slug?bet=encrypted_id
      return await decryptBetId(encryptedParam);
    }

    // Check if the ID looks like an encrypted string (contains URL-safe base64 chars)
    // Encrypted IDs will be longer and contain - or _ characters
    if (id.length > 20 && (id.includes('-') || id.includes('_'))) {
      return await decryptBetId(id);
    }

    // Otherwise, treat as regular numeric ID
    return id;
  }, [id, searchParams, decryptBetId]);

  const fetchBet = useCallback(async () => {
    try {
      setLoading(true);
      setDecryptionError(null);

      // Requirement 3.6: Resolve bet ID (decrypt if encrypted)
      const resolvedId = await resolveBetId();

      if (!resolvedId) {
        setDecryptionError('Invalid or expired shareable link');
        setLoading(false);
        return;
      }

      // Requirement 3.6: Fetch bet details using decrypted ID
      const response = await rustApiClient.get(`/api/v1/p2p-bets/${resolvedId}`);
      setBet(response.data);
    } catch (error: any) {
      console.error('Error fetching bet:', error);
      
      // Requirement 3.9: Handle invalid/expired URLs with error message
      if (error.response?.status === 404) {
        setDecryptionError('Bet not found. The link may be invalid or the bet may have been removed.');
      } else {
        handleError(error, { title: 'Failed to Load Bet' });
      }
    } finally {
      setLoading(false);
    }
  }, [resolveBetId]);

  useEffect(() => {
    fetchBet();
  }, [fetchBet]);

  // Subscribe to WebSocket updates for this bet
  // Requirement 8.6: Show live participant joins and outcome reports
  useEffect(() => {
    if (!bet) return;

    const unsubscribe = subscribeToBet(bet.id, (update: BetUpdate) => {
      handleBetUpdate(update);
    });

    return () => unsubscribe();
  }, [bet?.id, subscribeToBet]);

  const handleBetUpdate = useCallback((update: BetUpdate) => {
    setBet((prevBet) => {
      if (!prevBet) return prevBet;

      switch (update.type) {
        case 'participant_joined':
          // Add new participant to the bet
          return {
            ...prevBet,
            participants: [...prevBet.participants, update.data.participant],
            state: BetState.Active,
          };
        case 'outcome_reported':
          // Add outcome report
          return {
            ...prevBet,
            outcomeReports: [...prevBet.outcomeReports, update.data.report],
          };
        case 'outcome_verified':
          // Mark outcome as verified
          return {
            ...prevBet,
            state: BetState.Verified,
            verifiedOutcome: update.data.outcome,
          };
        case 'disputed':
          // Mark bet as disputed
          return {
            ...prevBet,
            state: BetState.Disputed,
            disputed: true,
          };
        case 'paid':
          // Mark bet as paid
          return {
            ...prevBet,
            state: BetState.Paid,
          };
        default:
          return prevBet;
      }
    });
  }, []);

  const getTimeRemaining = (endTime: Date) => {
    const now = new Date();
    const end = new Date(endTime);
    const diff = end.getTime() - now.getTime();

    if (diff <= 0) return 'Ended';

    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
    const seconds = Math.floor((diff % (1000 * 60)) / 1000);

    if (days > 0) return `${days}d ${hours}h ${minutes}m`;
    if (hours > 0) return `${hours}h ${minutes}m ${seconds}s`;
    if (minutes > 0) return `${minutes}m ${seconds}s`;
    return `${seconds}s`;
  };

  const formatAddress = (address: string) => {
    if (address.length <= 12) return address;
    return `${address.slice(0, 6)}...${address.slice(-6)}`;
  };

  const formatCurrency = (amount: number) => {
    if (amount >= 1000000) return `${(amount / 1000000).toFixed(1)}M`;
    if (amount >= 1000) return `${(amount / 1000).toFixed(0)}K`;
    return `${amount.toFixed(2)}`;
  };

  const handleJoinBet = async () => {
    if (!publicKey) {
      handleError(new Error('Wallet not connected'), {
        title: 'Wallet Required',
      });
      return;
    }

    if (!joinPosition || !stakeAmount) {
      handleError(new Error('Please select a position and enter stake amount'), {
        title: 'Invalid Input',
      });
      return;
    }

    setJoinLoading(true);

    try {
      // Convert stake to stroops
      const stakeInStroops = Math.floor(parseFloat(stakeAmount) * 10_000_000);

      // Call smart contract join_bet function via Freighter
      const contractId = import.meta.env.VITE_STELLAR_P2P_BET_CONTRACT_ID;

      if (!contractId) {
        throw new Error('P2P Bet contract ID not configured');
      }

      const StellarSdk = await import('@stellar/stellar-sdk');
      const server = new StellarSdk.Horizon.Server(
        import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
          ? 'https://horizon.stellar.org'
          : 'https://horizon-testnet.stellar.org'
      );

      const account = await server.loadAccount(publicKey);
      const contract = new StellarSdk.Contract(contractId);

      const transaction = new StellarSdk.TransactionBuilder(account, {
        fee: StellarSdk.BASE_FEE,
        networkPassphrase:
          import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
            ? StellarSdk.Networks.PUBLIC
            : StellarSdk.Networks.TESTNET,
      })
        .addOperation(
          contract.call(
            'join_bet',
            StellarSdk.Address.fromString(publicKey).toScVal(),
            StellarSdk.nativeToScVal(parseInt(id!), { type: 'u64' }),
            StellarSdk.nativeToScVal(joinPosition === 'Yes', { type: 'bool' }),
            StellarSdk.nativeToScVal(stakeInStroops, { type: 'i128' })
          )
        )
        .setTimeout(180)
        .build();

      const { signTransaction } = await import('@stellar/freighter-api');
      const { signedTxXdr, error: signError } = await signTransaction(
        transaction.toXDR(),
        {
          networkPassphrase:
            import.meta.env.VITE_STELLAR_NETWORK === 'mainnet'
              ? StellarSdk.Networks.PUBLIC
              : StellarSdk.Networks.TESTNET,
        }
      );

      if (signError) {
        throw new Error(`Transaction signing failed: ${signError}`);
      }

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

      // Call backend API to record participation
      await rustApiClient.post(`/api/v1/p2p-bets/${id}/join`, {
        position: joinPosition === 'Yes',
        stake: stakeInStroops,
        transaction_hash: result.hash,
      });

      handleSuccess('Joined Bet', 'You have successfully joined the bet!');
      setStakeAmount('');
      setJoinPosition(null);
      fetchBet();
    } catch (error: any) {
      console.error('Error joining bet:', error);
      handleError(error, {
        title: 'Failed to Join Bet',
        onRetry: handleJoinBet,
      });
    } finally {
      setJoinLoading(false);
    }
  };

  const handleShare = async (platform: 'twitter' | 'telegram' | 'copy') => {
    if (!bet) return;

    const shareUrl = bet.shareableUrl;
    const shareText = `Check out this bet: ${bet.question}`;

    try {
      switch (platform) {
        case 'twitter':
          window.open(
            `https://twitter.com/intent/tweet?text=${encodeURIComponent(
              shareText
            )}&url=${encodeURIComponent(shareUrl)}`,
            '_blank'
          );
          break;
        case 'telegram':
          window.open(
            `https://t.me/share/url?url=${encodeURIComponent(
              shareUrl
            )}&text=${encodeURIComponent(shareText)}`,
            '_blank'
          );
          break;
        case 'copy':
          await navigator.clipboard.writeText(shareUrl);
          handleSuccess('Link Copied', 'Shareable link copied to clipboard!');
          break;
      }
    } catch (error) {
      handleError(error, { title: 'Share Failed' });
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-white flex items-center justify-center">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
          <p className="mt-4 text-gray-600">Loading bet details...</p>
        </div>
      </div>
    );
  }

  // Requirement 3.9: Handle invalid/expired URLs with error message
  if (decryptionError || !bet) {
    return (
      <div className="min-h-screen bg-white flex items-center justify-center">
        <div className="text-center max-w-md mx-auto px-4">
          <div className="mb-4">
            <svg
              className="mx-auto h-16 w-16 text-red-500"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
          </div>
          <h2 className="text-2xl font-bold text-gray-900 mb-2">
            {decryptionError ? 'Invalid Link' : 'Bet Not Found'}
          </h2>
          <p className="text-gray-600 mb-6">
            {decryptionError || 
              "The bet you're looking for doesn't exist or has been removed."}
          </p>
          <button
            onClick={() => navigate('/bets')}
            className="bg-blue-600 text-white px-6 py-2 rounded-lg hover:bg-blue-700"
          >
            Back to Dashboard
          </button>
        </div>
      </div>
    );
  }

  const totalVolume = bet.participants.reduce((sum, p) => sum + p.stake, 0);
  const yesStake = bet.participants
    .filter((p) => p.position === 'Yes')
    .reduce((sum, p) => sum + p.stake, 0);
  const noStake = bet.participants
    .filter((p) => p.position === 'No')
    .reduce((sum, p) => sum + p.stake, 0);
  const yesProbability = totalVolume > 0 ? (yesStake / totalVolume) * 100 : 50;

  const isParticipant = publicKey
    ? bet.participants.some((p) => p.address === publicKey)
    : false;
  const canReportOutcome =
    isParticipant &&
    (bet.state === BetState.Ended ||
      new Date(bet.endTime).getTime() <= Date.now());

  return (
    <>
      <LoadingOverlay isVisible={joinLoading} message="Joining bet..." />

      {showOutcomeModal && publicKey && (
        <OutcomeReportingModal
          betId={bet.id}
          bet={bet}
          userAddress={publicKey}
          onClose={() => {
            setShowOutcomeModal(false);
            fetchBet();
          }}
        />
      )}

      <div className="min-h-screen bg-white py-8">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          {/* Back Button */}
          <button
            onClick={() => navigate('/bets')}
            className="mb-6 flex items-center text-gray-600 hover:text-gray-900"
          >
            <svg
              className="w-5 h-5 mr-2"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 19l-7-7 7-7"
              />
            </svg>
            Back to Dashboard
          </button>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            {/* Left Column - Main Content */}
            <div className="lg:col-span-2 space-y-6">
              {/* Bet Details Card */}
              <div className="bg-white border border-gray-200 rounded-xl p-6 shadow-sm">
                {/* State Badge */}
                <div className="mb-4">
                  <span
                    className={`inline-block px-3 py-1 rounded-full text-xs font-semibold ${
                      bet.state === BetState.Created
                        ? 'bg-blue-100 text-blue-700'
                        : bet.state === BetState.Active
                        ? 'bg-green-100 text-green-700'
                        : bet.state === BetState.Ended
                        ? 'bg-yellow-100 text-yellow-700'
                        : bet.state === BetState.Verified
                        ? 'bg-purple-100 text-purple-700'
                        : bet.state === BetState.Disputed
                        ? 'bg-red-100 text-red-700'
                        : 'bg-gray-100 text-gray-700'
                    }`}
                  >
                    {bet.state}
                  </span>
                  {bet.disputed && (
                    <span className="ml-2 inline-block px-3 py-1 rounded-full text-xs font-semibold bg-red-100 text-red-700">
                      Disputed
                    </span>
                  )}
                </div>

                {/* Question */}
                <h1 className="text-3xl font-bold text-gray-900 mb-4">
                  {bet.question}
                </h1>

                {/* Metadata */}
                <div className="grid grid-cols-2 gap-4 text-sm text-gray-600 mb-6">
                  <div>
                    <span className="text-gray-500">Creator:</span>{' '}
                    <span className="font-semibold text-gray-900">
                      {bet.creatorUsername}
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-500">Created:</span>{' '}
                    <span className="font-semibold text-gray-900">
                      {new Date(bet.createdAt).toLocaleDateString()}
                    </span>
                  </div>
                </div>

                {/* Countdown Timer */}
                <div className="bg-gradient-to-r from-purple-50 to-blue-50 rounded-lg p-4 mb-6">
                  <div className="text-center">
                    <div className="text-sm text-gray-600 mb-1">
                      {new Date(bet.endTime).getTime() > Date.now()
                        ? 'Ends In'
                        : 'Ended'}
                    </div>
                    <div className="text-3xl font-mono font-bold text-gray-900">
                      {getTimeRemaining(bet.endTime)}
                    </div>
                    <div className="text-xs text-gray-500 mt-1">
                      {new Date(bet.endTime).toLocaleString()}
                    </div>
                  </div>
                </div>

                {/* Probability Display */}
                <div className="mb-6">
                  <div className="flex justify-between items-center mb-2">
                    <span className="text-sm font-medium text-gray-700">
                      Current Probability
                    </span>
                  </div>
                  <div className="relative h-12 bg-gray-200 rounded-lg overflow-hidden">
                    <div
                      className="absolute left-0 top-0 h-full bg-gradient-to-r from-green-400 to-green-500 transition-all duration-500"
                      style={{ width: `${yesProbability}%` }}
                    ></div>
                    <div className="absolute inset-0 flex items-center justify-between px-4 text-sm font-semibold">
                      <span className="text-white drop-shadow">
                        Yes {yesProbability.toFixed(1)}%
                      </span>
                      <span className="text-gray-700 drop-shadow">
                        No {(100 - yesProbability).toFixed(1)}%
                      </span>
                    </div>
                  </div>
                </div>

                {/* Share Buttons */}
                <div className="border-t border-gray-200 pt-4">
                  <div className="text-sm font-medium text-gray-700 mb-3">
                    Share this bet
                  </div>
                  <div className="flex gap-3">
                    <button
                      onClick={() => handleShare('twitter')}
                      className="flex-1 bg-blue-500 hover:bg-blue-600 text-white py-2 px-4 rounded-lg transition-colors flex items-center justify-center gap-2"
                    >
                      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M23.953 4.57a10 10 0 01-2.825.775 4.958 4.958 0 002.163-2.723c-.951.555-2.005.959-3.127 1.184a4.92 4.92 0 00-8.384 4.482C7.69 8.095 4.067 6.13 1.64 3.162a4.822 4.822 0 00-.666 2.475c0 1.71.87 3.213 2.188 4.096a4.904 4.904 0 01-2.228-.616v.06a4.923 4.923 0 003.946 4.827 4.996 4.996 0 01-2.212.085 4.936 4.936 0 004.604 3.417 9.867 9.867 0 01-6.102 2.105c-.39 0-.779-.023-1.17-.067a13.995 13.995 0 007.557 2.209c9.053 0 13.998-7.496 13.998-13.985 0-.21 0-.42-.015-.63A9.935 9.935 0 0024 4.59z" />
                      </svg>
                      Twitter
                    </button>
                    <button
                      onClick={() => handleShare('telegram')}
                      className="flex-1 bg-blue-400 hover:bg-blue-500 text-white py-2 px-4 rounded-lg transition-colors flex items-center justify-center gap-2"
                    >
                      <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z" />
                      </svg>
                      Telegram
                    </button>
                    <button
                      onClick={() => handleShare('copy')}
                      className="flex-1 bg-gray-600 hover:bg-gray-700 text-white py-2 px-4 rounded-lg transition-colors flex items-center justify-center gap-2"
                    >
                      <svg
                        className="w-5 h-5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                        />
                      </svg>
                      Copy Link
                    </button>
                  </div>
                </div>
              </div>

              {/* Participants List */}
              <div className="bg-white border border-gray-200 rounded-xl p-6 shadow-sm">
                <h2 className="text-xl font-bold text-gray-900 mb-4">
                  Participants ({bet.participants.length})
                </h2>
                {bet.participants.length === 0 ? (
                  <p className="text-gray-500 text-center py-8">
                    No participants yet. Be the first to join!
                  </p>
                ) : (
                  <div className="space-y-3">
                    {bet.participants.map((participant, index) => (
                      <div
                        key={index}
                        className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                      >
                        <div className="flex items-center gap-3">
                          <div className="w-10 h-10 bg-gradient-to-br from-purple-400 to-blue-500 rounded-full flex items-center justify-center text-white font-bold">
                            {participant.username[0].toUpperCase()}
                          </div>
                          <div>
                            <div className="font-semibold text-gray-900">
                              {participant.username}
                            </div>
                            <div className="text-xs text-gray-500">
                              {formatAddress(participant.address)}
                            </div>
                          </div>
                        </div>
                        <div className="text-right">
                          <div
                            className={`font-semibold ${
                              participant.position === 'Yes'
                                ? 'text-green-600'
                                : 'text-red-600'
                            }`}
                          >
                            {participant.position}
                          </div>
                          <div className="text-sm text-gray-600">
                            {formatCurrency(participant.stake)} XLM
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Right Column - Trading & Stats */}
            <div className="space-y-6">
              {/* Stats Card */}
              <div className="bg-white border border-gray-200 rounded-xl p-6 shadow-sm">
                <h3 className="text-lg font-bold text-gray-900 mb-4">
                  Bet Statistics
                </h3>
                <div className="space-y-4">
                  <div>
                    <div className="text-sm text-gray-500 mb-1">Total Volume</div>
                    <div className="text-2xl font-bold text-gray-900">
                      {formatCurrency(totalVolume)} XLM
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-gray-500 mb-1">
                      Total Liquidity
                    </div>
                    <div className="text-2xl font-bold text-gray-900">
                      {formatCurrency(totalVolume)} XLM
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-4 pt-4 border-t border-gray-200">
                    <div>
                      <div className="text-sm text-gray-500 mb-1">Yes Pool</div>
                      <div className="text-lg font-semibold text-green-600">
                        {formatCurrency(yesStake)} XLM
                      </div>
                    </div>
                    <div>
                      <div className="text-sm text-gray-500 mb-1">No Pool</div>
                      <div className="text-lg font-semibold text-red-600">
                        {formatCurrency(noStake)} XLM
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Trading Interface */}
              {!isParticipant &&
                (bet.state === BetState.Created ||
                  bet.state === BetState.Active) &&
                new Date(bet.endTime).getTime() > Date.now() && (
                  <div className="bg-white border border-gray-200 rounded-xl p-6 shadow-sm">
                    <h3 className="text-lg font-bold text-gray-900 mb-4">
                      Join This Bet
                    </h3>

                    {!publicKey ? (
                      <div className="text-center py-4">
                        <p className="text-gray-600 mb-4">
                          Connect your wallet to join this bet
                        </p>
                        <button
                          onClick={() => navigate('/login')}
                          className="bg-blue-600 text-white px-6 py-2 rounded-lg hover:bg-blue-700"
                        >
                          Connect Wallet
                        </button>
                      </div>
                    ) : (
                      <>
                        <div className="space-y-3 mb-4">
                          <button
                            onClick={() => setJoinPosition('Yes')}
                            className={`w-full py-3 px-4 rounded-lg border-2 transition-colors ${
                              joinPosition === 'Yes'
                                ? 'border-green-500 bg-green-50 text-green-700 font-semibold'
                                : 'border-gray-300 hover:border-green-300'
                            }`}
                          >
                            Buy Yes ({yesProbability.toFixed(1)}%)
                          </button>
                          <button
                            onClick={() => setJoinPosition('No')}
                            className={`w-full py-3 px-4 rounded-lg border-2 transition-colors ${
                              joinPosition === 'No'
                                ? 'border-red-500 bg-red-50 text-red-700 font-semibold'
                                : 'border-gray-300 hover:border-red-300'
                            }`}
                          >
                            Buy No ({(100 - yesProbability).toFixed(1)}%)
                          </button>
                        </div>

                        <div className="mb-4">
                          <label className="block text-sm font-medium text-gray-700 mb-2">
                            Stake Amount (XLM)
                          </label>
                          <input
                            type="number"
                            value={stakeAmount}
                            onChange={(e) => setStakeAmount(e.target.value)}
                            placeholder="10"
                            step="0.01"
                            min="0"
                            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                          />
                        </div>

                        <button
                          onClick={handleJoinBet}
                          disabled={
                            joinLoading || !joinPosition || !stakeAmount
                          }
                          className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors font-semibold"
                        >
                          {joinLoading ? 'Joining...' : 'Join Bet'}
                        </button>
                      </>
                    )}
                  </div>
                )}

              {/* Outcome Reporting */}
              {canReportOutcome && (
                <div className="bg-white border border-gray-200 rounded-xl p-6 shadow-sm">
                  <h3 className="text-lg font-bold text-gray-900 mb-4">
                    Report Outcome
                  </h3>
                  <p className="text-sm text-gray-600 mb-4">
                    The bet has ended. As a participant, you can report the
                    outcome.
                  </p>
                  <button
                    onClick={() => setShowOutcomeModal(true)}
                    className="w-full bg-purple-600 text-white py-3 px-4 rounded-lg hover:bg-purple-700 transition-colors font-semibold"
                  >
                    Report Outcome
                  </button>
                </div>
              )}

              {/* Outcome Status */}
              {bet.state === BetState.Verified && (
                <div className="bg-green-50 border border-green-200 rounded-xl p-6">
                  <h3 className="text-lg font-bold text-green-900 mb-2">
                    Outcome Verified
                  </h3>
                  <p className="text-green-700">
                    Result: <span className="font-bold">{bet.verifiedOutcome ? 'Yes' : 'No'}</span>
                  </p>
                </div>
              )}

              {bet.state === BetState.Disputed && (
                <div className="bg-red-50 border border-red-200 rounded-xl p-6">
                  <h3 className="text-lg font-bold text-red-900 mb-2">
                    Outcome Disputed
                  </h3>
                  <p className="text-sm text-red-700">
                    Participants disagree on the outcome. Manual resolution
                    required.
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
