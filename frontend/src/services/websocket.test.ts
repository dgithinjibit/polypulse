/**
 * ============================================================
 * FILE: services/websocket.test.ts
 * PURPOSE: Unit tests for WebSocket service
 * ============================================================
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { WebSocketService } from './websocket';
import { BetUpdate } from '../types/p2p-bet';

// Mock WebSocket
class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readyState = MockWebSocket.CONNECTING;
  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;

  constructor(public url: string) {
    // Simulate connection opening after a short delay
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN;
      if (this.onopen) {
        this.onopen(new Event('open'));
      }
    }, 10);
  }

  send(data: string): void {
    // Mock send - do nothing
  }

  close(): void {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose(new CloseEvent('close'));
    }
  }
}

describe('WebSocketService', () => {
  let service: WebSocketService;
  let originalWebSocket: typeof WebSocket;

  beforeEach(() => {
    // Replace global WebSocket with mock
    originalWebSocket = global.WebSocket;
    global.WebSocket = MockWebSocket as any;
    
    service = new WebSocketService();
  });

  afterEach(() => {
    // Restore original WebSocket
    global.WebSocket = originalWebSocket;
    service.disconnect();
  });

  describe('Connection Management', () => {
    it('should start in disconnected state', () => {
      expect(service.getConnectionState()).toBe('disconnected');
    });

    it('should transition to connecting state when connect is called', () => {
      service.connect('test-token');
      expect(service.getConnectionState()).toBe('connecting');
    });

    it('should transition to connected state after connection opens', async () => {
      service.connect('test-token');
      
      // Wait for mock connection to open
      await new Promise(resolve => setTimeout(resolve, 20));
      
      expect(service.getConnectionState()).toBe('connected');
    });

    it('should not connect if already connected', async () => {
      service.connect('test-token');
      await new Promise(resolve => setTimeout(resolve, 20));
      
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      service.connect('test-token');
      
      expect(consoleSpy).toHaveBeenCalledWith('[WebSocket] Already connected or connecting');
      consoleSpy.mockRestore();
    });

    it('should disconnect and reset state', async () => {
      service.connect('test-token');
      await new Promise(resolve => setTimeout(resolve, 20));
      
      service.disconnect();
      
      expect(service.getConnectionState()).toBe('disconnected');
    });
  });

  describe('State Change Listeners', () => {
    it('should notify listeners of state changes', async () => {
      const listener = vi.fn();
      service.onStateChange(listener);
      
      service.connect('test-token');
      
      // Should be called for 'connecting' state
      expect(listener).toHaveBeenCalledWith('connecting');
      
      // Wait for connection to open
      await new Promise(resolve => setTimeout(resolve, 20));
      
      // Should be called for 'connected' state
      expect(listener).toHaveBeenCalledWith('connected');
    });

    it('should allow unsubscribing from state changes', () => {
      const listener = vi.fn();
      const unsubscribe = service.onStateChange(listener);
      
      unsubscribe();
      service.connect('test-token');
      
      expect(listener).not.toHaveBeenCalled();
    });
  });

  describe('Subscription Management', () => {
    it('should allow subscribing to bet updates', () => {
      const callback = vi.fn();
      const unsubscribe = service.subscribeToBet('bet-123', callback);
      
      expect(typeof unsubscribe).toBe('function');
    });

    it('should allow unsubscribing from bet updates', () => {
      const callback = vi.fn();
      const unsubscribe = service.subscribeToBet('bet-123', callback);
      
      unsubscribe();
      
      // Callback should not be called after unsubscribe
      // (would need to simulate message to fully test)
    });

    it('should support multiple subscribers for the same bet', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();
      
      service.subscribeToBet('bet-123', callback1);
      service.subscribeToBet('bet-123', callback2);
      
      // Both should be subscribed (would need message simulation to verify)
    });
  });

  describe('Message Handling', () => {
    it('should handle bet update messages', async () => {
      const callback = vi.fn();
      service.subscribeToBet('bet-123', callback);
      
      service.connect('test-token');
      await new Promise(resolve => setTimeout(resolve, 20));
      
      // Simulate receiving a bet update message
      const mockMessage: BetUpdate = {
        betId: 'bet-123',
        type: 'participant_joined',
        data: { participant: 'user-456' },
        timestamp: new Date(),
      };
      
      const messageEvent = new MessageEvent('message', {
        data: JSON.stringify({ type: 'bet_update', ...mockMessage }),
      });
      
      // Trigger message handler
      const ws = (service as any).ws;
      if (ws && ws.onmessage) {
        ws.onmessage(messageEvent);
      }
      
      expect(callback).toHaveBeenCalledWith(mockMessage);
    });

    it('should throttle updates to max 1 per second per bet', async () => {
      const callback = vi.fn();
      service.subscribeToBet('bet-123', callback);
      
      service.connect('test-token');
      await new Promise(resolve => setTimeout(resolve, 20));
      
      const mockMessage: BetUpdate = {
        betId: 'bet-123',
        type: 'participant_joined',
        data: {},
        timestamp: new Date(),
      };
      
      const ws = (service as any).ws;
      
      // Send first message
      if (ws && ws.onmessage) {
        ws.onmessage(new MessageEvent('message', {
          data: JSON.stringify({ type: 'bet_update', ...mockMessage }),
        }));
      }
      
      expect(callback).toHaveBeenCalledTimes(1);
      
      // Send second message immediately (should be throttled)
      if (ws && ws.onmessage) {
        ws.onmessage(new MessageEvent('message', {
          data: JSON.stringify({ type: 'bet_update', ...mockMessage }),
        }));
      }
      
      // Should still be 1 due to throttling
      expect(callback).toHaveBeenCalledTimes(1);
      
      // Wait for throttle period to pass
      await new Promise(resolve => setTimeout(resolve, 1100));
      
      // Send third message (should go through)
      if (ws && ws.onmessage) {
        ws.onmessage(new MessageEvent('message', {
          data: JSON.stringify({ type: 'bet_update', ...mockMessage }),
        }));
      }
      
      expect(callback).toHaveBeenCalledTimes(2);
    });

    it('should handle pong messages without error', async () => {
      service.connect('test-token');
      await new Promise(resolve => setTimeout(resolve, 20));
      
      const ws = (service as any).ws;
      
      // Should not throw
      if (ws && ws.onmessage) {
        ws.onmessage(new MessageEvent('message', {
          data: JSON.stringify({ type: 'pong' }),
        }));
      }
    });

    it('should handle error messages', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      service.connect('test-token');
      await new Promise(resolve => setTimeout(resolve, 20));
      
      const ws = (service as any).ws;
      
      if (ws && ws.onmessage) {
        ws.onmessage(new MessageEvent('message', {
          data: JSON.stringify({ type: 'error', message: 'Test error' }),
        }));
      }
      
      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
    });
  });

  describe('Reconnection Logic', () => {
    it('should attempt reconnection on disconnect', async () => {
      vi.useFakeTimers();
      
      service.connect('test-token');
      await vi.advanceTimersByTimeAsync(20);
      
      expect(service.getConnectionState()).toBe('connected');
      
      // Simulate disconnect
      const ws = (service as any).ws;
      if (ws && ws.onclose) {
        ws.onclose(new CloseEvent('close'));
      }
      
      expect(service.getConnectionState()).toBe('reconnecting');
      
      vi.useRealTimers();
    });

    it('should use exponential backoff for reconnection', async () => {
      vi.useFakeTimers();
      
      service.connect('test-token');
      await vi.advanceTimersByTimeAsync(20);
      
      // First disconnect - should reconnect after 1s
      const ws1 = (service as any).ws;
      if (ws1 && ws1.onclose) {
        ws1.onclose(new CloseEvent('close'));
      }
      
      expect(service.getConnectionState()).toBe('reconnecting');
      
      // Advance by 1 second (first backoff)
      await vi.advanceTimersByTimeAsync(1000);
      
      // Should attempt reconnection
      expect(service.getConnectionState()).toBe('connecting');
      
      vi.useRealTimers();
    });
  });
});
