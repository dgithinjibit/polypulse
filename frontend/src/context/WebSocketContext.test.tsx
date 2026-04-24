/**
 * ============================================================
 * FILE: WebSocketContext.test.tsx
 * PURPOSE: Unit tests for WebSocketContext provider
 * ============================================================
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { WebSocketProvider, useWebSocket } from './WebSocketContext';
import { AuthProvider } from './AuthContext';

// Mock the websocket service
vi.mock('../services/websocket', () => ({
  websocketService: {
    connect: vi.fn(),
    disconnect: vi.fn(),
    subscribeToBet: vi.fn(() => vi.fn()),
    onStateChange: vi.fn(() => vi.fn()),
    getConnectionState: vi.fn(() => 'disconnected'),
  },
}));

// Mock the auth context
vi.mock('./AuthContext', () => ({
  AuthProvider: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  useAuth: vi.fn(() => ({
    user: null,
    loading: false,
    login: vi.fn(),
    logout: vi.fn(),
    refreshUser: vi.fn(),
  })),
}));

// Test component that uses the WebSocket context
function TestComponent() {
  const { connectionState, isConnected } = useWebSocket();
  
  return (
    <div>
      <div data-testid="connection-state">{connectionState}</div>
      <div data-testid="is-connected">{isConnected ? 'true' : 'false'}</div>
    </div>
  );
}

describe('WebSocketContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should provide WebSocket context to children', () => {
    render(
      <AuthProvider>
        <WebSocketProvider>
          <TestComponent />
        </WebSocketProvider>
      </AuthProvider>
    );

    expect(screen.getByTestId('connection-state')).toBeInTheDocument();
    expect(screen.getByTestId('is-connected')).toBeInTheDocument();
  });

  it('should throw error when useWebSocket is used outside provider', () => {
    // Suppress console.error for this test
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => {
      render(<TestComponent />);
    }).toThrow('useWebSocket must be used within WebSocketProvider');

    consoleSpy.mockRestore();
  });

  it('should expose connection state', () => {
    render(
      <AuthProvider>
        <WebSocketProvider>
          <TestComponent />
        </WebSocketProvider>
      </AuthProvider>
    );

    const connectionState = screen.getByTestId('connection-state');
    expect(connectionState.textContent).toBe('disconnected');
  });

  it('should expose isConnected boolean', () => {
    render(
      <AuthProvider>
        <WebSocketProvider>
          <TestComponent />
        </WebSocketProvider>
      </AuthProvider>
    );

    const isConnected = screen.getByTestId('is-connected');
    expect(isConnected.textContent).toBe('false');
  });
});
