import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { BrowserRouter, MemoryRouter, Route, Routes } from 'react-router-dom';
import BetDetailPage from '../pages/BetDetailPage';
import { StellarWalletProvider } from '../context/StellarWalletContext';
import { WebSocketProvider } from '../context/WebSocketContext';
import { AuthProvider } from '../context/AuthContext';
import * as EncryptionService from '../services/encryption';
import rustApiClient from '../config/api';

// Mock the API client
vi.mock('../config/api', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

// Mock the encryption service
vi.mock('../services/encryption', () => ({
  EncryptionService: {
    decryptBetId: vi.fn(),
    encryptBetId: vi.fn(),
    generateShareableUrl: vi.fn(),
  },
}));

// Mock the error handler
vi.mock('../lib/error-handler', () => ({
  handleError: vi.fn(),
  handleSuccess: vi.fn(),
}));

const mockBet = {
  id: '123',
  creator: 'GABC123',
  creatorUsername: 'testuser',
  question: 'Will it rain tomorrow?',
  stakeAmount: 100,
  endTime: new Date(Date.now() + 86400000).toISOString(),
  state: 'Active',
  createdAt: new Date().toISOString(),
  shareableUrl: 'https://example.com/bet/encrypted123',
  participants: [],
  outcomeReports: [],
  disputed: false,
};

const renderWithProviders = (ui: React.ReactElement, { route = '/bet/123' } = {}) => {
  return render(
    <MemoryRouter initialEntries={[route]}>
      <AuthProvider>
        <WebSocketProvider>
          <StellarWalletProvider>
            <Routes>
              <Route path="/bet/:id" element={ui} />
            </Routes>
          </StellarWalletProvider>
        </WebSocketProvider>
      </AuthProvider>
    </MemoryRouter>
  );
};

describe('BetDetailPage - Shareable URL Handling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should load bet with regular numeric ID', async () => {
    // Mock API response
    vi.mocked(rustApiClient.get).mockResolvedValueOnce({
      data: mockBet,
    });

    renderWithProviders(<BetDetailPage />, { route: '/bet/123' });

    // Should show loading state
    expect(screen.getByText(/loading bet details/i)).toBeInTheDocument();

    // Wait for bet to load
    await waitFor(() => {
      expect(screen.getByText(mockBet.question)).toBeInTheDocument();
    });

    // Should call API with numeric ID
    expect(rustApiClient.get).toHaveBeenCalledWith('/api/v1/p2p-bets/123');
  });

  it('should decrypt and load bet with encrypted ID in URL path', async () => {
    const encryptedId = 'abc123def456_encrypted-id';
    const decryptedId = '123';

    // Mock decryption
    vi.mocked(EncryptionService.EncryptionService.decryptBetId).mockResolvedValueOnce(
      decryptedId
    );

    // Mock API response
    vi.mocked(rustApiClient.get).mockResolvedValueOnce({
      data: mockBet,
    });

    renderWithProviders(<BetDetailPage />, { route: `/bet/${encryptedId}` });

    // Wait for decryption and bet load
    await waitFor(() => {
      expect(screen.getByText(mockBet.question)).toBeInTheDocument();
    });

    // Should decrypt the ID
    expect(EncryptionService.EncryptionService.decryptBetId).toHaveBeenCalledWith(
      encryptedId,
      expect.any(String)
    );

    // Should call API with decrypted ID
    expect(rustApiClient.get).toHaveBeenCalledWith(`/api/v1/p2p-bets/${decryptedId}`);
  });

  it('should decrypt and load bet with encrypted ID in query parameter', async () => {
    const encryptedId = 'abc123def456_encrypted-id';
    const decryptedId = '123';

    // Mock decryption
    vi.mocked(EncryptionService.EncryptionService.decryptBetId).mockResolvedValueOnce(
      decryptedId
    );

    // Mock API response
    vi.mocked(rustApiClient.get).mockResolvedValueOnce({
      data: mockBet,
    });

    renderWithProviders(<BetDetailPage />, {
      route: `/bet/some-slug?bet=${encryptedId}`,
    });

    // Wait for decryption and bet load
    await waitFor(() => {
      expect(screen.getByText(mockBet.question)).toBeInTheDocument();
    });

    // Should decrypt the ID from query param
    expect(EncryptionService.EncryptionService.decryptBetId).toHaveBeenCalledWith(
      encryptedId,
      expect.any(String)
    );

    // Should call API with decrypted ID
    expect(rustApiClient.get).toHaveBeenCalledWith(`/api/v1/p2p-bets/${decryptedId}`);
  });

  it('should show error message for invalid encrypted ID', async () => {
    const encryptedId = 'invalid_encrypted_id-with-dashes';

    // Mock decryption to return null (decryption failed)
    vi.mocked(EncryptionService.EncryptionService.decryptBetId).mockResolvedValueOnce(
      null as any
    );

    renderWithProviders(<BetDetailPage />, { route: `/bet/${encryptedId}` });

    // Wait for error message - when decryption returns null, we show "Invalid or expired shareable link"
    await waitFor(
      () => {
        const heading = screen.getByRole('heading', { level: 2 });
        expect(heading).toHaveTextContent(/invalid link/i);
      },
      { timeout: 2000 }
    );
  });

  it('should show error message when bet is not found', async () => {
    const encryptedId = 'abc123def456_encrypted-id-long';
    const decryptedId = '999';

    // Mock successful decryption
    vi.mocked(EncryptionService.EncryptionService.decryptBetId).mockResolvedValueOnce(
      decryptedId
    );

    // Mock API 404 error - bet not found
    vi.mocked(rustApiClient.get).mockRejectedValueOnce({
      response: { status: 404 },
    });

    renderWithProviders(<BetDetailPage />, { route: `/bet/${encryptedId}` });

    // Wait for error message - when API returns 404, we show "Bet not found..."
    await waitFor(
      () => {
        const text = screen.getByText(/bet not found/i);
        expect(text).toBeInTheDocument();
      },
      { timeout: 2000 }
    );
  });

  it('should show back to dashboard button on error', async () => {
    const encryptedId = 'invalid_encrypted_id';

    // Mock decryption failure
    vi.mocked(EncryptionService.EncryptionService.decryptBetId).mockResolvedValueOnce(
      null as any
    );

    renderWithProviders(<BetDetailPage />, { route: `/bet/${encryptedId}` });

    // Wait for error message
    await waitFor(() => {
      expect(screen.getByText(/back to dashboard/i)).toBeInTheDocument();
    });

    const backButton = screen.getByRole('button', { name: /back to dashboard/i });
    expect(backButton).toBeInTheDocument();
  });
});
