import { useEffect, useState } from 'react';

interface LeaderboardEntry {
  rank: number;
  userId: number;
  username: string;
  avatarUrl?: string;
  score: number;
  totalBets: number;
  level: number;
}

type LeaderboardCategory = 'earners' | 'predictors' | 'active';

export function Leaderboard() {
  const [category, setCategory] = useState<LeaderboardCategory>('earners');
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchLeaderboard();
  }, [category]);

  const fetchLeaderboard = async () => {
    setLoading(true);
    try {
      const response = await fetch(`/api/v1/leaderboard/${category}`);
      if (!response.ok) throw new Error('Failed to fetch leaderboard');
      const data = await response.json();
      setEntries(data);
    } catch (error) {
      console.error('Error fetching leaderboard:', error);
    } finally {
      setLoading(false);
    }
  };

  const getMedal = (rank: number): string => {
    switch (rank) {
      case 1: return '🥇';
      case 2: return '🥈';
      case 3: return '🥉';
      default: return '';
    }
  };

  const getLevelBadge = (level: number): string => {
    switch (level) {
      case 1: return '🥉 Bronze';
      case 2: return '🥈 Silver';
      case 3: return '🥇 Gold';
      case 4: return '💎 Diamond';
      default: return '';
    }
  };

  const getCategoryLabel = (cat: LeaderboardCategory): string => {
    switch (cat) {
      case 'earners': return 'Top Earners';
      case 'predictors': return 'Best Predictors';
      case 'active': return 'Most Active';
    }
  };

  const getScoreLabel = (cat: LeaderboardCategory): string => {
    switch (cat) {
      case 'earners': return 'Total Earnings';
      case 'predictors': return 'XP';
      case 'active': return 'Bets';
    }
  };

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-6">🏆 Leaderboard</h2>

      {/* Category Tabs */}
      <div className="flex gap-2 mb-6 border-b">
        {(['earners', 'predictors', 'active'] as LeaderboardCategory[]).map((cat) => (
          <button
            key={cat}
            onClick={() => setCategory(cat)}
            className={`px-4 py-2 font-medium transition-colors ${
              category === cat
                ? 'text-blue-600 border-b-2 border-blue-600'
                : 'text-gray-600 hover:text-gray-900'
            }`}
          >
            {getCategoryLabel(cat)}
          </button>
        ))}
      </div>

      {/* Leaderboard List */}
      {loading ? (
        <div className="text-center py-8 text-gray-500">Loading...</div>
      ) : entries.length === 0 ? (
        <div className="text-center py-8 text-gray-500">No data yet</div>
      ) : (
        <div className="space-y-3">
          {entries.map((entry) => (
            <div
              key={entry.userId}
              className={`flex items-center gap-4 p-4 rounded-lg transition-colors ${
                entry.rank <= 3
                  ? 'bg-gradient-to-r from-yellow-50 to-orange-50 border border-yellow-200'
                  : 'bg-gray-50 hover:bg-gray-100'
              }`}
            >
              {/* Rank */}
              <div className="flex-shrink-0 w-12 text-center">
                {entry.rank <= 3 ? (
                  <span className="text-2xl">{getMedal(entry.rank)}</span>
                ) : (
                  <span className="text-lg font-bold text-gray-600">#{entry.rank}</span>
                )}
              </div>

              {/* Avatar */}
              <div className="flex-shrink-0">
                {entry.avatarUrl ? (
                  <img
                    src={entry.avatarUrl}
                    alt={entry.username}
                    className="w-12 h-12 rounded-full"
                  />
                ) : (
                  <div className="w-12 h-12 rounded-full bg-blue-500 flex items-center justify-center text-white font-bold">
                    {entry.username.charAt(0).toUpperCase()}
                  </div>
                )}
              </div>

              {/* User Info */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <p className="font-semibold text-gray-900 truncate">{entry.username}</p>
                  <span className="text-xs px-2 py-1 rounded bg-blue-100 text-blue-700">
                    {getLevelBadge(entry.level)}
                  </span>
                </div>
                <p className="text-sm text-gray-600">
                  {entry.totalBets} {entry.totalBets === 1 ? 'bet' : 'bets'}
                </p>
              </div>

              {/* Score */}
              <div className="text-right">
                <p className="text-lg font-bold text-gray-900">
                  {category === 'earners'
                    ? `${(entry.score / 10_000_000).toFixed(2)} XLM`
                    : entry.score.toLocaleString()}
                </p>
                <p className="text-xs text-gray-600">{getScoreLabel(category)}</p>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
