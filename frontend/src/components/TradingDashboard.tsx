import { useState } from 'react';
import { calculateProbability } from '../lib/AMMEngine';

interface Market {
  id: string;
  question: string;
  category: string;
  yesPool: number;
  noPool: number;
  volume: number;
  liquidity: number;
}

// Sample markets data
const sampleMarkets: Market[] = [
  {
    id: '1',
    question: 'Will HBAR reach $1.00 by end of 2026?',
    category: 'Crypto',
    yesPool: 4600,
    noPool: 5400,
    volume: 125000,
    liquidity: 100000,
  },
  {
    id: '2',
    question: 'Will Kenya launch CBDC on Hedera in 2025?',
    category: 'Finance',
    yesPool: 6800,
    noPool: 3200,
    volume: 65000,
    liquidity: 100000,
  },
  {
    id: '3',
    question: 'Will Hedera process 1B+ transactions in Q1 2025?',
    category: 'Technology',
    yesPool: 2900,
    noPool: 7100,
    volume: 180000,
    liquidity: 100000,
  },
  {
    id: '4',
    question: 'Will Stellar XLM reach $0.50 by June 2025?',
    category: 'Crypto',
    yesPool: 5500,
    noPool: 4500,
    volume: 95000,
    liquidity: 100000,
  },
  {
    id: '5',
    question: 'Will Hedera partner with major African bank in 2025?',
    category: 'Finance',
    yesPool: 4200,
    noPool: 5800,
    volume: 72000,
    liquidity: 100000,
  },
  {
    id: '6',
    question: 'Will Hedera governance council add African member in 2025?',
    category: 'Technology',
    yesPool: 3800,
    noPool: 6200,
    volume: 58000,
    liquidity: 100000,
  },
];

export default function TradingDashboard() {
  const [markets] = useState<Market[]>(sampleMarkets);

  const totalVolume = markets.reduce((sum, m) => sum + m.volume, 0);
  const totalLiquidity = markets.reduce((sum, m) => sum + m.liquidity, 0);

  const formatCurrency = (amount: number) => {
    if (amount >= 1000000) return `$${(amount / 1000000).toFixed(1)}M`;
    if (amount >= 1000) return `$${(amount / 1000).toFixed(0)}K`;
    return `$${amount}`;
  };

  const getCategoryColor = (category: string) => {
    switch (category) {
      case 'Crypto': return 'bg-purple-100 text-purple-700';
      case 'Finance': return 'bg-blue-100 text-blue-700';
      case 'Technology': return 'bg-green-100 text-green-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  };

  return (
    <div className="min-h-screen bg-white">
      {/* Top Stats Bar */}
      <div className="bg-gradient-to-r from-purple-50 to-blue-50 border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                <span className="text-sm font-medium text-gray-700">Live Markets</span>
              </div>
              <div className="text-sm text-gray-600">
                <span className="font-semibold text-gray-900">{markets.length}</span> Active
              </div>
              <div className="text-sm text-gray-600">
                Volume: <span className="font-semibold text-gray-900">{formatCurrency(totalVolume)}</span>
              </div>
              <div className="text-sm text-gray-600">
                Liquidity: <span className="font-semibold text-gray-900">{formatCurrency(totalLiquidity)}</span>
              </div>
            </div>
            <div className="flex items-center gap-2 text-sm text-gray-600">
              <svg className="w-4 h-4 text-purple-600" fill="currentColor" viewBox="0 0 20 20">
                <path d="M10 2a8 8 0 100 16 8 8 0 000-16zM9 9a1 1 0 112 0v4a1 1 0 11-2 0V9zm1-4a1 1 0 100 2 1 1 0 000-2z"/>
              </svg>
              <span>Powered by Stellar</span>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-gray-900 mb-2">Active Markets</h1>
          <p className="text-gray-600">Trade on {markets.length} live prediction markets</p>
        </div>

        {/* Markets Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {markets.map((market) => {
            const probability = calculateProbability(market.yesPool, market.noPool);
            const yesPrice = (probability / 100).toFixed(2);
            const noPrice = ((100 - probability) / 100).toFixed(2);

            return (
              <div
                key={market.id}
                className="bg-gray-50 border border-gray-200 rounded-xl p-6 hover:shadow-lg hover:-translate-y-1 transition-all duration-200 cursor-pointer"
              >
                {/* Category Badge */}
                <div className="mb-3">
                  <span className={`inline-block px-3 py-1 rounded-full text-xs font-semibold ${getCategoryColor(market.category)}`}>
                    {market.category}
                  </span>
                </div>

                {/* Question */}
                <h3 className="text-lg font-semibold text-gray-900 mb-4 line-clamp-2 min-h-[3.5rem]">
                  {market.question}
                </h3>

                {/* Probability Display */}
                <div className="mb-4">
                  <div className={`text-4xl font-mono font-bold ${probability > 50 ? 'text-green-600' : 'text-red-600'}`}>
                    {probability.toFixed(1)}%
                  </div>
                  <div className="text-sm text-gray-500 mt-1">chance</div>
                </div>

                {/* Stats */}
                <div className="flex justify-between text-sm text-gray-600 mb-4 pb-4 border-b border-gray-200">
                  <div>
                    <div className="text-xs text-gray-500">Volume</div>
                    <div className="font-semibold text-gray-900">{formatCurrency(market.volume)}</div>
                  </div>
                  <div className="text-right">
                    <div className="text-xs text-gray-500">Liquidity</div>
                    <div className="font-semibold text-gray-900">{formatCurrency(market.liquidity)}</div>
                  </div>
                </div>

                {/* Trading Buttons */}
                <div className="grid grid-cols-2 gap-3">
                  <button className="bg-green-500 hover:bg-green-600 text-white font-semibold py-3 px-4 rounded-lg transition-colors">
                    <div className="text-xs opacity-90">Buy Yes</div>
                    <div className="text-sm">¢{(parseFloat(yesPrice) * 100).toFixed(0)}</div>
                  </button>
                  <button className="bg-red-500 hover:bg-red-600 text-white font-semibold py-3 px-4 rounded-lg transition-colors">
                    <div className="text-xs opacity-90">Buy No</div>
                    <div className="text-sm">¢{(parseFloat(noPrice) * 100).toFixed(0)}</div>
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
