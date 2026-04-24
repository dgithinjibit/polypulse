/**
 * Unit tests for BetDashboard component
 * Tests rendering, filtering, sorting, and search functionality
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import BetDashboard from '../pages/BetDashboard';
import { BetState } from '../types/p2p-bet';

// ── Mocks ──────────────────────────────────────────────────────────────────

const mockNavigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

const mockPublicKey = 'GBTEST1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCDE';
vi.mock('../context/StellarWalletContext', () => ({
  useStellarWallet: () => ({
    publicKey: mockPublicKey,
    isConnected: true,
  }),
}));

// Mock fetch for API calls
global.fetch = vi.fn();

// ── Helpers ────────────────────────────────────────────────────────────────

const mockBets = [
  {
    id: '1',
    creator: mockPublicKey,
    creatorUsername: 'testuser',
    question: 'Will it rain tomorrow?',
    stakeAmount: 100,
    endTime: new Date(Date.now() + 86400000), // 1 day from now
    state: BetState.Active,
    createdAt: new Date(),
    shareableUrl: 'https://example.com/bet/1',
    participants: [
      {
        address: mockPublicKey,
        username: 'testuser',
        position: 'Yes' as const,
        stake: 50,
        joinedAt: new Date(),
        hasReported: false,
      },
    ],
    outcomeReports: [],
    disputed: false,
  },
  {
    id: '2',
    creator: mockPublicKey,
    creatorUsername: 'testuser2',
    question: 'Will Bitcoin reach $100k?',
    stakeAmount: 200,
    endTime: new Date(Date.now() + 172800000), // 2 days from now
    state: BetState.Created,
    createdAt: new Date(),
    shareableUrl: 'https://example.com/bet/2',
    participants: [],
    outcomeReports: [],
    disputed: false,
  },
];

function renderWithRouter(ui: React.ReactElement) {
  return render(<BrowserRouter>{ui}</BrowserRouter>);
}

// ── Tests ──────────────────────────────────────────────────────────────────

describe('BetDashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    (global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => mockBets,
    });
  });

  describe('rendering', () => {
    it('displays the dashboard title', async () => {
      renderWithRouter(<BetDashboard />);

      expect(screen.getByText('P2P Betting Dashboard')).toBeInTheDocument();
    });

    it('displays "Create Your Bet" button when wallet is connected', async () => {
      renderWithRouter(<BetDashboard />);

      expect(screen.getByText('Create Your Bet')).toBeInTheDocument();
    });

    it('displays search input', async () => {
      renderWithRouter(<BetDashboard />);

      expect(screen.getByPlaceholderText('Search bets...')).toBeInTheDocument();
    });

    it('displays filter dropdown', async () => {
      renderWithRouter(<BetDashboard />);

      const filterSelect = screen.getByDisplayValue('All');
      expect(filterSelect).toBeInTheDocument();
    });

    it('displays sort dropdown', async () => {
      renderWithRouter(<BetDashboard />);

      const sortSelect = screen.getByDisplayValue('Newest');
      expect(sortSelect).toBeInTheDocument();
    });
  });

  describe('bet loading', () => {
    it('shows loading state initially', () => {
      renderWithRouter(<BetDashboard />);

      expect(screen.getByText('Loading bets...')).toBeInTheDocument();
    });

    it('displays bets after loading', async () => {
      renderWithRouter(<BetDashboard />);

      await waitFor(() => {
        expect(screen.getByText('Will it rain tomorrow?')).toBeInTheDocument();
        expect(screen.getByText('Will Bitcoin reach $100k?')).toBeInTheDocument();
      });
    });

    it('displays "No bets found" when no bets are returned', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => [],
      });

      renderWithRouter(<BetDashboard />);

      await waitFor(() => {
        expect(screen.getByText('No bets found')).toBeInTheDocument();
      });
    });
  });

  describe('localStorage preferences', () => {
    it('saves filter preference to localStorage', async () => {
      renderWithRouter(<BetDashboard />);

      await waitFor(() => {
        const saved = localStorage.getItem('betDashboardPreferences');
        expect(saved).toBeTruthy();
        const preferences = JSON.parse(saved!);
        expect(preferences.filter).toBe('All');
      });
    });

    it('saves sort preference to localStorage', async () => {
      renderWithRouter(<BetDashboard />);

      await waitFor(() => {
        const saved = localStorage.getItem('betDashboardPreferences');
        expect(saved).toBeTruthy();
        const preferences = JSON.parse(saved!);
        expect(preferences.sort).toBe('Newest');
      });
    });

    it('loads preferences from localStorage on mount', async () => {
      const savedPreferences = {
        filter: 'Active',
        sort: 'Volume',
      };
      localStorage.setItem('betDashboardPreferences', JSON.stringify(savedPreferences));

      renderWithRouter(<BetDashboard />);

      await waitFor(() => {
        expect(screen.getByDisplayValue('Active')).toBeInTheDocument();
        expect(screen.getByDisplayValue('Volume')).toBeInTheDocument();
      });
    });
  });

  describe('stats display', () => {
    it('displays total number of bets', async () => {
      renderWithRouter(<BetDashboard />);

      await waitFor(() => {
        expect(screen.getByText('2')).toBeInTheDocument();
      });
    });

    it('displays total volume', async () => {
      renderWithRouter(<BetDashboard />);

      await waitFor(() => {
        // Total volume is 50 XLM from participants
        expect(screen.getByText(/50\.00 XLM/)).toBeInTheDocument();
      });
    });
  });
});
