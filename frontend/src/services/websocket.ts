/**
 * ============================================================
 * FILE: services/websocket.ts
 * PURPOSE: WebSocket service for real-time P2P bet updates
 *          Handles connection, subscription, message routing,
 *          and automatic reconnection with exponential backoff.
 * 
 * REQUIREMENTS: 8.1, 8.2, 8.7, 8.8, 8.9, 8.10
 * ============================================================
 */

import { BetUpdate } from '../types/p2p-bet';

// ============================================================
// TYPES
// ============================================================

/**
 * Server-to-client message types from backend
 */
interface ServerMessage {
  type: 'price_update' | 'poll_resolved' | 'comment_added' | 'notification' | 
        'challenge_invite' | 'pong' | 'error' | 'bet_update';
  [key: string]: any;
}

/**
 * Notification message from server
 */
interface NotificationMessage {
  type: 'notification';
  notification_id: number;
  notification_type: string;
  message: string;
  actor_id?: string;
  timestamp: number;
}

/**
 * Client-to-server message types
 */
interface ClientMessage {
  type: 'subscribe_bet' | 'unsubscribe_bet' | 'ping';
  bet_id?: string;
}

/**
 * Connection state
 */
type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

  /**
   * Subscription callback function
   */
  type SubscriptionCallback = (data: BetUpdate) => void;
  
  /**
   * Notification callback function
   */
  type NotificationCallback = (notification: NotificationMessage) => void;

// ============================================================
// WEBSOCKET SERVICE CLASS
// ============================================================

export class WebSocketService {
  private ws: WebSocket | null = null;
  private connectionState: ConnectionState = 'disconnected';
  private reconnectAttempts = 0;
  private readonly maxReconnectAttempts = 5;
  private readonly baseReconnectDelay = 1000; // 1 second
  private reconnectTimeout: NodeJS.Timeout | null = null;
  private heartbeatInterval: NodeJS.Timeout | null = null;
  
  /**
   * Map of bet_id -> Set of callback functions
   * Allows multiple components to subscribe to the same bet
   */
  private subscriptions = new Map<string, Set<SubscriptionCallback>>();
  
  /**
   * Set of notification callback functions
   */
  private notificationCallbacks = new Set<NotificationCallback>();
  
  /**
   * Throttle map: bet_id -> last update timestamp
   * Ensures max 1 update per second per bet (Requirement 8.10)
   */
  private lastUpdateTime = new Map<string, number>();
  private readonly updateThrottleMs = 1000; // 1 second
  
  /**
   * Connection state change listeners
   */
  private stateListeners = new Set<(state: ConnectionState) => void>();
  
  /**
   * Stored token for reconnection
   */
  private token: string | null = null;

  // ============================================================
  // CONNECTION MANAGEMENT
  // ============================================================

  /**
   * Connect to WebSocket server with JWT authentication
   * Requirement 8.1: Establish WebSocket connection after wallet connection
   */
  connect(token: string): void {
    if (this.connectionState === 'connected' || this.connectionState === 'connecting') {
      console.warn('[WebSocket] Already connected or connecting');
      return;
    }

    this.token = token;
    this.connectionState = 'connecting';
    this.notifyStateChange();

    try {
      // Construct WebSocket URL with token as query parameter
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      const wsUrl = `${protocol}//${host}/ws?token=${encodeURIComponent(token)}`;

      this.ws = new WebSocket(wsUrl);
      this.setupEventHandlers();
    } catch (error) {
      console.error('[WebSocket] Connection error:', error);
      this.handleDisconnect();
    }
  }

  /**
   * Disconnect from WebSocket server
   */
  disconnect(): void {
    this.clearReconnectTimeout();
    this.clearHeartbeat();
    
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    
    this.connectionState = 'disconnected';
    this.reconnectAttempts = 0;
    this.notifyStateChange();
  }

  /**
   * Get current connection state
   * Requirement 8.7: Display connection status indicator
   */
  getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  /**
   * Add connection state change listener
   */
  onStateChange(listener: (state: ConnectionState) => void): () => void {
    this.stateListeners.add(listener);
    return () => this.stateListeners.delete(listener);
  }

  // ============================================================
  // SUBSCRIPTION MANAGEMENT
  // ============================================================

  /**
   * Subscribe to bet updates
   * Requirement 8.2: Subscribe to bet updates for all displayed markets
   * 
   * @param betId - The bet ID to subscribe to
   * @param callback - Function to call when updates are received
   * @returns Unsubscribe function
   */
  subscribeToBet(betId: string, callback: SubscriptionCallback): () => void {
    // Add callback to subscriptions
    if (!this.subscriptions.has(betId)) {
      this.subscriptions.set(betId, new Set());
    }
    this.subscriptions.get(betId)!.add(callback);

    // Send subscription message to server if connected
    if (this.connectionState === 'connected') {
      this.sendMessage({
        type: 'subscribe_bet',
        bet_id: betId,
      });
    }

    // Return unsubscribe function
    return () => this.unsubscribeFromBet(betId, callback);
  }

  /**
   * Subscribe to notification updates
   * 
   * @param callback - Function to call when notifications are received
   * @returns Unsubscribe function
   */
  subscribeToNotifications(callback: NotificationCallback): () => void {
    this.notificationCallbacks.add(callback);
    return () => this.notificationCallbacks.delete(callback);
  }

  /**
   * Unsubscribe from bet updates
   */
  private unsubscribeFromBet(betId: string, callback: SubscriptionCallback): void {
    const callbacks = this.subscriptions.get(betId);
    if (!callbacks) return;

    callbacks.delete(callback);

    // If no more callbacks for this bet, unsubscribe from server
    if (callbacks.size === 0) {
      this.subscriptions.delete(betId);
      this.lastUpdateTime.delete(betId);

      if (this.connectionState === 'connected') {
        this.sendMessage({
          type: 'unsubscribe_bet',
          bet_id: betId,
        });
      }
    }
  }

  // ============================================================
  // MESSAGE HANDLING
  // ============================================================

  /**
   * Handle incoming WebSocket messages
   * Requirement 8.3-8.6: Update displays based on message type
   * Requirement 8.10: Throttle updates to max 1 per second per bet
   */
  private handleMessage(event: MessageEvent): void {
    try {
      const message: ServerMessage = JSON.parse(event.data);

      // Route message based on type
      switch (message.type) {
        case 'bet_update':
          this.handleBetUpdate(message as any);
          break;
        case 'notification':
          this.handleNotification(message as NotificationMessage);
          break;
        case 'pong':
          // Heartbeat response - connection is alive
          break;
        case 'error':
          console.error('[WebSocket] Server error:', message);
          break;
        default:
          // Ignore other message types (poll updates, etc.)
          break;
      }
    } catch (error) {
      console.error('[WebSocket] Failed to parse message:', error);
    }
  }

  /**
   * Handle bet update messages with throttling
   */
  private handleBetUpdate(update: BetUpdate): void {
    const { betId } = update;
    const now = Date.now();
    const lastUpdate = this.lastUpdateTime.get(betId) || 0;

    // Throttle: only process if at least 1 second has passed since last update
    if (now - lastUpdate < this.updateThrottleMs) {
      return;
    }

    this.lastUpdateTime.set(betId, now);

    // Notify all subscribers for this bet
    const callbacks = this.subscriptions.get(betId);
    if (callbacks) {
      callbacks.forEach(callback => {
        try {
          callback(update);
        } catch (error) {
          console.error('[WebSocket] Callback error:', error);
        }
      });
    }
  }

  /**
   * Handle notification messages
   */
  private handleNotification(notification: NotificationMessage): void {
    // Notify all notification subscribers
    this.notificationCallbacks.forEach(callback => {
      try {
        callback(notification);
      } catch (error) {
        console.error('[WebSocket] Notification callback error:', error);
      }
    });
  }

  /**
   * Send message to server
   */
  private sendMessage(message: ClientMessage): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  // ============================================================
  // EVENT HANDLERS
  // ============================================================

  /**
   * Set up WebSocket event handlers
   */
  private setupEventHandlers(): void {
    if (!this.ws) return;

    this.ws.onopen = () => {
      console.log('[WebSocket] Connected');
      this.connectionState = 'connected';
      this.reconnectAttempts = 0;
      this.notifyStateChange();
      
      // Restore subscriptions after reconnection
      this.restoreSubscriptions();
      
      // Start heartbeat
      this.startHeartbeat();
    };

    this.ws.onmessage = (event) => {
      this.handleMessage(event);
    };

    this.ws.onerror = (error) => {
      console.error('[WebSocket] Error:', error);
    };

    this.ws.onclose = () => {
      console.log('[WebSocket] Disconnected');
      this.handleDisconnect();
    };
  }

  /**
   * Handle disconnection and attempt reconnection
   * Requirement 8.8: Attempt automatic reconnection when disconnected
   */
  private handleDisconnect(): void {
    this.clearHeartbeat();
    this.ws = null;

    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.connectionState = 'reconnecting';
      this.notifyStateChange();
      this.scheduleReconnect();
    } else {
      this.connectionState = 'disconnected';
      this.notifyStateChange();
      console.error('[WebSocket] Max reconnection attempts reached');
    }
  }

  /**
   * Schedule reconnection with exponential backoff
   * Requirement 8.8: Exponential backoff for reconnection
   */
  private scheduleReconnect(): void {
    this.clearReconnectTimeout();

    // Calculate delay with exponential backoff: 1s, 2s, 4s, 8s, 16s
    const delay = this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts);
    
    console.log(`[WebSocket] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts + 1}/${this.maxReconnectAttempts})`);

    this.reconnectTimeout = setTimeout(() => {
      this.reconnectAttempts++;
      if (this.token) {
        this.connect(this.token);
      }
    }, delay);
  }

  /**
   * Restore subscriptions after reconnection
   */
  private restoreSubscriptions(): void {
    for (const betId of this.subscriptions.keys()) {
      this.sendMessage({
        type: 'subscribe_bet',
        bet_id: betId,
      });
    }
  }

  // ============================================================
  // HEARTBEAT
  // ============================================================

  /**
   * Start heartbeat to keep connection alive
   */
  private startHeartbeat(): void {
    this.clearHeartbeat();
    
    this.heartbeatInterval = setInterval(() => {
      if (this.connectionState === 'connected') {
        this.sendMessage({ type: 'ping' });
      }
    }, 30000); // Ping every 30 seconds
  }

  /**
   * Clear heartbeat interval
   */
  private clearHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  // ============================================================
  // UTILITY METHODS
  // ============================================================

  /**
   * Clear reconnection timeout
   */
  private clearReconnectTimeout(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
  }

  /**
   * Notify all state change listeners
   */
  private notifyStateChange(): void {
    this.stateListeners.forEach(listener => {
      try {
        listener(this.connectionState);
      } catch (error) {
        console.error('[WebSocket] State listener error:', error);
      }
    });
  }
}

// ============================================================
// SINGLETON INSTANCE
// ============================================================

/**
 * Singleton instance of WebSocket service
 * Use this instance throughout the application
 */
export const websocketService = new WebSocketService();
