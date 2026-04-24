import { useNavigate } from 'react-router-dom';
import { Bet, BetState } from '../types/p2p-bet';

interface MarketCardProps {
  bet: Bet;
}

export function MarketCard({ bet }: MarketCardProps) {
  const navigate = useNavigate();

  const formatCurrency = (amount: number) => {
    if (amount >= 1000000) return `${(amount / 1000000).toFixed(1)}M`;
    if (amount >= 1000) return `${(amount / 1000).toFixed(0)}K`;
    return `${amount.toFixed(2)}`;
  };

  const getStateColor = (state: BetState) => {
    switch (state) {
      case BetState.Created:
        return 'bg-blue-100 text-blue-700';
      case BetState.Active:
        return 'bg-green-100 text-green-700';
      case BetState.Ended:
        return 'bg-yellow-100 text-yellow-700';
      case BetState.Verified:
        return 'bg-purple-100 text-purple-700';
      case BetState.Disputed:
        return 'bg-red-100 text-red-700';
      case BetState.Paid:
        return 'bg-gray-100 text-gray-700';
      case BetState.Cancelled:
        return 'bg-gray-100 text-gray-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  };

  const getTimeRemaining = (endTime: Date) => {
    const now = new Date();
    const end = new Date(endTime);
    const diff = end.getTime() - now.getTime();

    if (diff <= 0) return 'Ended';

    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  };

  const totalVolume = bet.participants.reduce((sum, p) => sum + p.stake, 0);
  const yesStake = bet.participants
    .filter((p) => p.position === 'Yes')
    .reduce((sum, p) => sum + p.stake, 0);
  const noStake = bet.participants
    .filter((p) => p.position === 'No')
    .reduce((sum, p) => sum + p.stake, 0);

  const yesProbability = totalVolume > 0 ? (yesStake / totalVolume) * 100 : 50;

  return (
    <div
      className="bg-gray-50 border border-gray-200 rounded-xl p-6 hover:shadow-lg hover:-translate-y-1 transition-all duration-200 cursor-pointer"
      onClick={() => navigate(`/bet/${bet.id}`)}
    >
      {/* State Badge */}
      <div className="mb-3">
        <span
          className={`inline-block px-3 py-1 rounded-full text-xs font-semibold ${getStateColor(
            bet.state
          )}`}
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
      <h3 className="text-lg font-semibold text-gray-900 mb-4 line-clamp-2 min-h-[3.5rem]">
        {bet.question}
      </h3>

      {/* Probability Display */}
      <div className="mb-4">
        <div
          className={`text-4xl font-mono font-bold ${
            yesProbability > 50 ? 'text-green-600' : 'text-red-600'
          }`}
        >
          {yesProbability.toFixed(1)}%
        </div>
        <div className="text-sm text-gray-500 mt-1">chance of Yes</div>
      </div>

      {/* Stats */}
      <div className="flex justify-between text-sm text-gray-600 mb-4 pb-4 border-b border-gray-200">
        <div>
          <div className="text-xs text-gray-500">Volume</div>
          <div className="font-semibold text-gray-900">{formatCurrency(totalVolume)} XLM</div>
        </div>
        <div className="text-right">
          <div className="text-xs text-gray-500">Ends In</div>
          <div className="font-semibold text-gray-900">{getTimeRemaining(bet.endTime)}</div>
        </div>
      </div>

      {/* Participants */}
      <div className="flex justify-between text-sm text-gray-600 mb-4">
        <div>
          <div className="text-xs text-gray-500">Participants</div>
          <div className="font-semibold text-gray-900">{bet.participants.length}</div>
        </div>
        <div className="text-right">
          <div className="text-xs text-gray-500">Creator</div>
          <div className="font-semibold text-gray-900">{bet.creatorUsername}</div>
        </div>
      </div>

      {/* Trading Buttons */}
      <div className="grid grid-cols-2 gap-3">
        <button
          className="bg-green-500 hover:bg-green-600 text-white font-semibold py-3 px-4 rounded-lg transition-colors"
          onClick={(e) => {
            e.stopPropagation();
            navigate(`/bet/${bet.id}?position=yes`);
          }}
        >
          <div className="text-xs opacity-90">Join Yes</div>
          <div className="text-sm">{yesProbability.toFixed(0)}%</div>
        </button>
        <button
          className="bg-red-500 hover:bg-red-600 text-white font-semibold py-3 px-4 rounded-lg transition-colors"
          onClick={(e) => {
            e.stopPropagation();
            navigate(`/bet/${bet.id}?position=no`);
          }}
        >
          <div className="text-xs opacity-90">Join No</div>
          <div className="text-sm">{(100 - yesProbability).toFixed(0)}%</div>
        </button>
      </div>
    </div>
  );
}
