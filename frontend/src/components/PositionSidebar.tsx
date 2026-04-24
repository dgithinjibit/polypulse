import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Position } from '../types/p2p-bet';

export function PositionSidebar() {
  const navigate = useNavigate();
  const [positions, setPositions] = useState<Position[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchPositions();
  }, []);

  const fetchPositions = async () => {
    try {
      const response = await fetch('/api/v1/p2p-bets/my-positions');
      if (!response.ok) {
        throw new Error('Failed to fetch positions');
      }
      const data = await response.json();
      setPositions(data);
    } catch (error) {
      console.error('Error fetching positions:', error);
    } finally {
      setLoading(false);
    }
  };

  const totalValue = positions.reduce((sum, p) => sum + p.currentValue, 0);
  const totalProfitLoss = positions.reduce((sum, p) => sum + p.profitLoss, 0);
  const profitLossPercentage = totalValue > 0 ? (totalProfitLoss / totalValue) * 100 : 0;

  const truncateQuestion = (question: string, maxLength: number = 40): string => {
    if (question.length <= maxLength) return question;
    return question.substring(0, maxLength) + '...';
  };

  if (loading) {
    return (
      <div className="bg-white rounded-lg shadow p-4">
        <h3 className="text-lg font-bold mb-4">My Positions</h3>
        <p className="text-gray-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow p-4">
      <h3 className="text-lg font-bold mb-4">My Positions</h3>

      {positions.length === 0 ? (
        <p className="text-gray-500 text-sm">No active positions</p>
      ) : (
        <>
          <div className="mb-4 pb-4 border-b">
            <div className="flex justify-between items-center mb-2">
              <span className="text-sm text-gray-600">Total Value</span>
              <span className="font-bold">{totalValue.toFixed(2)} XLM</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-sm text-gray-600">Profit/Loss</span>
              <span
                className={`font-bold ${
                  totalProfitLoss >= 0 ? 'text-green-600' : 'text-red-600'
                }`}
              >
                {totalProfitLoss >= 0 ? '+' : ''}
                {totalProfitLoss.toFixed(2)} XLM ({profitLossPercentage.toFixed(1)}%)
              </span>
            </div>
          </div>

          <div className="space-y-3 max-h-96 overflow-y-auto">
            {positions.map((position) => (
              <div
                key={position.betId}
                className="p-3 border border-gray-200 rounded-md hover:bg-gray-50 cursor-pointer transition-colors"
                onClick={() => navigate(`/bet/${position.betId}`)}
              >
                <p className="text-sm font-medium mb-1">
                  {truncateQuestion(position.question)}
                </p>
                <div className="flex justify-between items-center text-xs text-gray-600 mb-2">
                  <span
                    className={`px-2 py-1 rounded ${
                      position.position === 'Yes'
                        ? 'bg-green-100 text-green-700'
                        : 'bg-red-100 text-red-700'
                    }`}
                  >
                    {position.position}
                  </span>
                  <span>{position.stake.toFixed(2)} XLM</span>
                </div>
                <div className="flex justify-between items-center text-xs">
                  <span className="text-gray-600">Current Value</span>
                  <span className="font-medium">{position.currentValue.toFixed(2)} XLM</span>
                </div>
                <div className="flex justify-between items-center text-xs">
                  <span className="text-gray-600">P/L</span>
                  <span
                    className={`font-medium ${
                      position.profitLoss >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}
                  >
                    {position.profitLoss >= 0 ? '+' : ''}
                    {position.profitLoss.toFixed(2)} XLM
                  </span>
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
