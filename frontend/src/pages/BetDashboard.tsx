import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Bet, BetState } from '../types/p2p-bet';
import { MarketCard } from '../components/MarketCard';
import { PositionSidebar } from '../components/PositionSidebar';
import { BetCreationForm } from '../components/BetCreationForm';
import { useStellarWallet } from '../context/StellarWalletContext';

type FilterOption = 'All' | 'Active' | 'Ending Soon' | 'Ended';
type SortOption = 'Volume' | 'Liquidity' | 'Newest' | 'Ending Soon';

interface FilterPreferences {
  filter: FilterOption;
  sort: SortOption;
}

export default function BetDashboard() {
  const navigate = useNavigate();
  const { publicKey } = useStellarWallet();
  
  const [bets, setBets] = useState<Bet[]>([]);
  const [filteredBets, setFilteredBets] = useState<Bet[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [filter, setFilter] = useState<FilterOption>('All');
  const [sort, setSort] = useState<SortOption>('Newest');
  const [showCreateForm, setShowCreateForm] = useState(false);

  // Load preferences from localStorage
  useEffect(() => {
    const savedPreferences = localStorage.getItem('betDashboardPreferences');
    if (savedPreferences) {
      try {
        const preferences: FilterPreferences = JSON.parse(savedPreferences);
        setFilter(preferences.filter);
        setSort(preferences.sort);
      } catch (error) {
        console.error('Error loading preferences:', error);
      }
    }
  }, []);

  // Save preferences to localStorage
  useEffect(() => {
    const preferences: FilterPreferences = { filter, sort };
    localStorage.setItem('betDashboardPreferences', JSON.stringify(preferences));
  }, [filter, sort]);

  // Fetch bets from API
  useEffect(() => {
    fetchBets();
  }, []);

  const fetchBets = async () => {
    try {
      setLoading(true);
      const response = await fetch('/api/v1/p2p-bets');
      if (!response.ok) {
        throw new Error('Failed to fetch bets');
      }
      const data = await response.json();
      setBets(data);
    } catch (error) {
      console.error('Error fetching bets:', error);
    } finally {
      setLoading(false);
    }
  };

  // Debounced search with 300ms delay
  useEffect(() => {
    const timer = setTimeout(() => {
      applyFiltersAndSort();
    }, 300);

    return () => clearTimeout(timer);
  }, [searchQuery, filter, sort, bets]);

  const applyFiltersAndSort = useCallback(() => {
    let result = [...bets];

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter((bet) =>
        bet.question.toLowerCase().includes(query)
      );
    }

    // Apply status filter
    if (filter !== 'All') {
      const now = new Date();
      const oneHour = 60 * 60 * 1000;

      switch (filter) {
        case 'Active':
          result = result.filter(
            (bet) =>
              bet.state === BetState.Active || bet.state === BetState.Created
          );
          break;
        case 'Ending Soon':
          result = result.filter((bet) => {
            const endTime = new Date(bet.endTime);
            const timeRemaining = endTime.getTime() - now.getTime();
            return (
              timeRemaining > 0 &&
              timeRemaining <= oneHour &&
              (bet.state === BetState.Active || bet.state === BetState.Created)
            );
          });
          break;
        case 'Ended':
          result = result.filter(
            (bet) =>
              bet.state === BetState.Ended ||
              bet.state === BetState.Verified ||
              bet.state === BetState.Paid ||
              bet.state === BetState.Disputed
          );
          break;
      }
    }

    // Apply sort
    switch (sort) {
      case 'Volume':
        result.sort((a, b) => {
          const volumeA = a.participants.reduce((sum, p) => sum + p.stake, 0);
          const volumeB = b.participants.reduce((sum, p) => sum + p.stake, 0);
          return volumeB - volumeA;
        });
        break;
      case 'Liquidity':
        result.sort((a, b) => {
          const liquidityA = a.participants.reduce((sum, p) => sum + p.stake, 0);
          const liquidityB = b.participants.reduce((sum, p) => sum + p.stake, 0);
          return liquidityB - liquidityA;
        });
        break;
      case 'Newest':
        result.sort(
          (a, b) =>
            new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
        );
        break;
      case 'Ending Soon':
        result.sort(
          (a, b) =>
            new Date(a.endTime).getTime() - new Date(b.endTime).getTime()
        );
        break;
    }

    setFilteredBets(result);
  }, [searchQuery, filter, sort, bets]);

  const handleCreateSuccess = (betId: string, shareableUrl: string) => {
    setShowCreateForm(false);
    fetchBets();
    navigate(`/bet/${betId}`);
  };

  const totalVolume = bets.reduce(
    (sum, bet) => sum + bet.participants.reduce((s, p) => s + p.stake, 0),
    0
  );

  const formatCurrency = (amount: number) => {
    if (amount >= 1000000) return `${(amount / 1000000).toFixed(1)}M`;
    if (amount >= 1000) return `${(amount / 1000).toFixed(0)}K`;
    return `${amount.toFixed(2)}`;
  };

  if (showCreateForm) {
    return (
      <div className="min-h-screen bg-white py-8">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <BetCreationForm
            onSuccess={handleCreateSuccess}
            onCancel={() => setShowCreateForm(false)}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-white">
      {/* Top Stats Bar */}
      <div className="bg-gradient-to-r from-purple-50 to-blue-50 border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                <span className="text-sm font-medium text-gray-700">
                  Live P2P Bets
                </span>
              </div>
              <div className="text-sm text-gray-600">
                <span className="font-semibold text-gray-900">{bets.length}</span>{' '}
                Active
              </div>
              <div className="text-sm text-gray-600">
                Volume:{' '}
                <span className="font-semibold text-gray-900">
                  {formatCurrency(totalVolume)} XLM
                </span>
              </div>
            </div>
            <div className="flex items-center gap-2 text-sm text-gray-600">
              <svg
                className="w-4 h-4 text-purple-600"
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <path d="M10 2a8 8 0 100 16 8 8 0 000-16zM9 9a1 1 0 112 0v4a1 1 0 11-2 0V9zm1-4a1 1 0 100 2 1 1 0 000-2z" />
              </svg>
              <span>Powered by Stellar</span>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="flex gap-6">
          {/* Left Column - Bets */}
          <div className="flex-1">
            {/* Header with Create Button */}
            <div className="mb-6 flex items-center justify-between">
              <div>
                <h1 className="text-3xl font-bold text-gray-900 mb-2">
                  P2P Betting Dashboard
                </h1>
                <p className="text-gray-600">
                  Create and join peer-to-peer bets on any event
                </p>
              </div>
              {publicKey && (
                <button
                  onClick={() => setShowCreateForm(true)}
                  className="bg-blue-600 hover:bg-blue-700 text-white font-semibold py-3 px-6 rounded-lg transition-colors shadow-md"
                >
                  Create Your Bet
                </button>
              )}
            </div>

            {/* Search and Filters */}
            <div className="mb-6 flex gap-4">
              {/* Search Input */}
              <div className="flex-1">
                <input
                  type="text"
                  placeholder="Search bets..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              {/* Filter Dropdown */}
              <select
                value={filter}
                onChange={(e) => setFilter(e.target.value as FilterOption)}
                className="px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 bg-white"
              >
                <option value="All">All</option>
                <option value="Active">Active</option>
                <option value="Ending Soon">Ending Soon</option>
                <option value="Ended">Ended</option>
              </select>

              {/* Sort Dropdown */}
              <select
                value={sort}
                onChange={(e) => setSort(e.target.value as SortOption)}
                className="px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 bg-white"
              >
                <option value="Volume">Volume</option>
                <option value="Liquidity">Liquidity</option>
                <option value="Newest">Newest</option>
                <option value="Ending Soon">Ending Soon</option>
              </select>
            </div>

            {/* Bets Grid */}
            {loading ? (
              <div className="text-center py-12">
                <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
                <p className="mt-4 text-gray-600">Loading bets...</p>
              </div>
            ) : filteredBets.length === 0 ? (
              <div className="text-center py-12 bg-gray-50 rounded-lg border border-gray-200">
                <svg
                  className="mx-auto h-12 w-12 text-gray-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
                <h3 className="mt-4 text-lg font-medium text-gray-900">
                  No bets found
                </h3>
                <p className="mt-2 text-gray-600">
                  {searchQuery
                    ? 'Try adjusting your search or filters'
                    : 'Be the first to create a bet!'}
                </p>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {filteredBets.map((bet) => (
                  <MarketCard key={bet.id} bet={bet} />
                ))}
              </div>
            )}
          </div>

          {/* Right Column - Position Sidebar */}
          {publicKey && (
            <div className="w-80 flex-shrink-0">
              <PositionSidebar />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
