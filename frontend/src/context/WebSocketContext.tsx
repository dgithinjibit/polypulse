/**
 * ============================================================
 * FILE: WebSocketContext.tsx
 * PURPOSE: React Context for WebSocket connection management.
 *          Provides real-time bet updates to all components.
 *          Automatically connects when user is authenticated.
 *
 * PROVIDES:
 *   - connectionState: Current WebSocket connection status
 *   - subscribeToBet: Subscribe to updates for a specific bet
 *   - isConnected: Boolean indicating if WebSocket is connected
 *
 * REQUIREMENTS: 8.1, 8.2
 * ============================================================
 */

import React, { createContext, useContext, useEffect, useState, useCallback, ReactNode } from 'react';
import { websocketService } from '../services/websocket';
import { BetUpdate } from '../types/p2p-bet';
import { useAuth } from './AuthContext';

// ============================================================
// TYPE: ConnectionState
// PURPOSE: Represents the current WebSocket connection status
// ============================================================
type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

// ============================================================
// TYPE: WebSocketContextValue
// PURPOSE: Defines what this context exposes to consuming components
// ============================================================
interface WebSocketContextValue {
  connectionState: ConnectionState;
  isConnected: boolean;
  subscribeToBet: (betId: string, callback: (data: BetUpdate) => void) => () => void;
}

// Create the context with null default (throws if used outside provider)
const WebSocketContext = createContext<WebSocketContextValue | null>(null);

// ============================================================
// COMPONENT: WebSocketProvider
// PURPOSE: Provides WebSocket connection state to all child components.
//          Automatically connects when user is authenticated.
//          Placed in App.tsx to wrap the entire application.
// PARAM children: Child components that will have access to WebSocket state
// ============================================================
export function WebSocketProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');

  // ============================================================
  // EFFECT: Connect/disconnect WebSocket based on auth state
  // PURPOSE: Automatically establish WebSocket connection when user logs in
  //          and disconnect when user logs out.
  // REQUIREMENT: 8.1 - Establish WebSocket connection after wallet connection
  // ============================================================
  useEffect(() => {
    // Subscribe to connection state changes
    const unsubscribe = websocketService.onStateChange((state) => {
      setConnectionState(state);
    });

    // Connect if user is authenticated
    if (user) {
      const token = localStorage.getItem('access_token');
      if (token) {
        websocketService.connect(token);
      }
    } else {
      // Disconnect if user logs out
      websocketService.disconnect();
    }

    // Cleanup on unmount
    return () => {
      unsubscribe();
      websocketService.disconnect();
    };
  }, [user]);

  // ============================================================
  // FUNCTION: subscribeToBet
  // PURPOSE: Subscribe to real-time updates for a specific bet
  // REQUIREMENT: 8.2 - Subscribe to bet updates for all displayed markets
  // PARAM betId: The bet ID to subscribe to
  // PARAM callback: Function to call when updates are received
  // RETURNS: Unsubscribe function
  // ============================================================
  const subscribeToBet = useCallback(
    (betId: string, callback: (data: BetUpdate) => void) => {
      return websocketService.subscribeToBet(betId, callback);
    },
    []
  );

  // Compute isConnected for convenience
  const isConnected = connectionState === 'connected';

  // ============================================================
  // PROVIDER RENDER
  // PURPOSE: Provides WebSocket state and actions to all child components
  // ============================================================
  return (
    <WebSocketContext.Provider
      value={{
        connectionState,
        isConnected,
        subscribeToBet,
      }}
    >
      {children}
    </WebSocketContext.Provider>
  );
}

// ============================================================
// HOOK: useWebSocket
// PURPOSE: Custom hook for consuming the WebSocketContext.
//          Throws a helpful error if used outside the provider.
// RETURNS: WebSocketContextValue - connection state and subscribe function
// USAGE: const { isConnected, subscribeToBet } = useWebSocket()
// ============================================================
export const useWebSocket = (): WebSocketContextValue => {
  const ctx = useContext(WebSocketContext);

  // If ctx is null, this hook was called outside of WebSocketProvider
  if (!ctx) {
    throw new Error('useWebSocket must be used within WebSocketProvider');
  }

  return ctx;
};
