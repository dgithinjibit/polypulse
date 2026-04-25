// ============================================================
// FILE: ws/p2p_bets.rs
// PURPOSE: WebSocket handler for real-time P2P bet updates.
//          Clients connect with a JWT token, subscribe to specific bet IDs,
//          and receive live updates via Redis Pub/Sub.
//
// FLOW:
//   1. Client connects: GET /ws/p2p-bets?token=<jwt>
//   2. JWT is validated; connection is registered
//   3. Client sends { "type": "subscribe_bet", "bet_id": 42 }
//   4. Server subscribes to Redis channel "bet_updates:42"
//   5. When any API endpoint publishes to that channel, the message is
//      forwarded to all subscribed WebSocket clients
//   6. On disconnect, all Redis subscriptions are cleaned up
//
// REDIS CHANNEL NAMING: "bet_updates:{bet_id}"
//
// Requirements: 8.1, 8.2
// ============================================================

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use deadpool_redis::redis::AsyncCommands;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{errors::AppError, middleware::auth::Claims, state::AppState};

// ─── Constants ────────────────────────────────────────────────────────────────

const CHANNEL_CAPACITY: usize = 64;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const MAX_BET_SUBSCRIPTIONS: usize = 50;

// ─── Redis channel helper ─────────────────────────────────────────────────────

/// Returns the Redis Pub/Sub channel name for a given bet ID.
pub fn bet_channel(bet_id: i64) -> String {
    format!("bet_updates:{}", bet_id)
}

// ─── Message Types ────────────────────────────────────────────────────────────

/// Messages sent from the client to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum P2PClientMessage {
    SubscribeBet { bet_id: i64 },
    UnsubscribeBet { bet_id: i64 },
    Ping,
}

/// Messages sent from the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum P2PServerMessage {
    /// A bet update forwarded from Redis Pub/Sub.
    BetUpdate(BetUpdatePayload),
    /// Acknowledgement that a subscription was registered.
    Subscribed { bet_id: i64 },
    /// Acknowledgement that a subscription was removed.
    Unsubscribed { bet_id: i64 },
    /// Heartbeat response.
    Pong,
    /// Error message.
    Error { message: String },
}

/// The payload published to Redis and forwarded to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetUpdatePayload {
    pub bet_id: i64,
    /// One of: "participant_joined", "outcome_reported", "outcome_verified",
    ///         "disputed", "paid", "cancelled"
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

// ─── P2P Bet Connection Registry ─────────────────────────────────────────────

/// Tracks which WebSocket connections are subscribed to which bet IDs.
/// This is separate from the main `ConnectionRegistry` in `ws/mod.rs` so that
/// P2P bet subscriptions don't interfere with poll subscriptions.
#[derive(Clone)]
pub struct P2PConnectionRegistry {
    /// connection_id → set of subscribed bet IDs
    subscriptions: Arc<RwLock<HashMap<Uuid, HashSet<i64>>>>,
    /// bet_id → set of connection IDs
    bet_subscribers: Arc<RwLock<HashMap<i64, HashSet<Uuid>>>>,
    /// connection_id → broadcast sender
    senders: Arc<RwLock<HashMap<Uuid, broadcast::Sender<P2PServerMessage>>>>,
}

impl P2PConnectionRegistry {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            bet_subscribers: Arc::new(RwLock::new(HashMap::new())),
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, conn_id: Uuid, sender: broadcast::Sender<P2PServerMessage>) {
        let mut senders = self.senders.write().await;
        senders.insert(conn_id, sender);
        let mut subs = self.subscriptions.write().await;
        subs.insert(conn_id, HashSet::new());
    }

    pub async fn unregister(&self, conn_id: Uuid) -> HashSet<i64> {
        // Remove sender
        let mut senders = self.senders.write().await;
        senders.remove(&conn_id);
        drop(senders);

        // Remove from all bet subscriber sets and collect which bets were subscribed
        let mut subs = self.subscriptions.write().await;
        let bet_ids = subs.remove(&conn_id).unwrap_or_default();
        drop(subs);

        let mut bet_subs = self.bet_subscribers.write().await;
        for &bet_id in &bet_ids {
            if let Some(set) = bet_subs.get_mut(&bet_id) {
                set.remove(&conn_id);
                if set.is_empty() {
                    bet_subs.remove(&bet_id);
                }
            }
        }

        bet_ids
    }

    pub async fn subscribe_bet(&self, conn_id: Uuid, bet_id: i64) -> Result<(), String> {
        let mut subs = self.subscriptions.write().await;
        let conn_subs = subs.entry(conn_id).or_insert_with(HashSet::new);
        if conn_subs.len() >= MAX_BET_SUBSCRIPTIONS {
            return Err(format!(
                "Max bet subscriptions ({}) reached",
                MAX_BET_SUBSCRIPTIONS
            ));
        }
        conn_subs.insert(bet_id);
        drop(subs);

        let mut bet_subs = self.bet_subscribers.write().await;
        bet_subs.entry(bet_id).or_insert_with(HashSet::new).insert(conn_id);

        Ok(())
    }

    pub async fn unsubscribe_bet(&self, conn_id: Uuid, bet_id: i64) {
        let mut subs = self.subscriptions.write().await;
        if let Some(set) = subs.get_mut(&conn_id) {
            set.remove(&bet_id);
        }
        drop(subs);

        let mut bet_subs = self.bet_subscribers.write().await;
        if let Some(set) = bet_subs.get_mut(&bet_id) {
            set.remove(&conn_id);
            if set.is_empty() {
                bet_subs.remove(&bet_id);
            }
        }
    }

    /// Broadcast a message to all connections subscribed to a bet.
    pub async fn broadcast_to_bet(&self, bet_id: i64, msg: P2PServerMessage) {
        let bet_subs = self.bet_subscribers.read().await;
        let conn_ids: Vec<Uuid> = bet_subs
            .get(&bet_id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        drop(bet_subs);

        let senders = self.senders.read().await;
        for conn_id in conn_ids {
            if let Some(tx) = senders.get(&conn_id) {
                let _ = tx.send(msg.clone());
            }
        }
    }

    /// Returns true if any connection is still subscribed to the given bet.
    pub async fn has_subscribers(&self, bet_id: i64) -> bool {
        let bet_subs = self.bet_subscribers.read().await;
        bet_subs.get(&bet_id).map(|s| !s.is_empty()).unwrap_or(false)
    }
}

// ─── Query Parameters ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct P2PWsQuery {
    pub token: String,
}

// ─── WebSocket Handler ────────────────────────────────────────────────────────

/// GET /ws/p2p-bets?token=<jwt>
///
/// Upgrades the HTTP connection to a WebSocket and starts the P2P bet
/// real-time update session.
pub async fn p2p_bets_ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<P2PWsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let claims = match validate_ws_token(&query.token, &state) {
        Ok(c) => c,
        Err(e) => {
            warn!("P2P WS auth failed: {}", e);
            return axum::http::Response::builder()
                .status(401)
                .body(axum::body::Body::from("Invalid or missing token"))
                .unwrap()
                .into_response();
        }
    };

    let user_id = claims.sub;
    info!("P2P WS connection established: user={}", user_id);

    ws.on_upgrade(move |socket| handle_p2p_websocket(socket, user_id, state))
}

// ─── Connection Handler ───────────────────────────────────────────────────────

async fn handle_p2p_websocket(socket: WebSocket, user_id: Uuid, state: AppState) {
    let conn_id = Uuid::new_v4();

    // Per-connection broadcast channel
    let (tx, _) = broadcast::channel::<P2PServerMessage>(CHANNEL_CAPACITY);

    // Register in the P2P registry
    state
        .p2p_registry()
        .register(conn_id, tx.clone())
        .await;

    info!(
        "P2P WS registered: conn_id={}, user_id={}",
        conn_id, user_id
    );

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut rx = tx.subscribe();

    // Heartbeat task — sends Pong every HEARTBEAT_INTERVAL to keep the
    // connection alive through proxies and load balancers.
    let heartbeat_tx = tx.clone();
    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        loop {
            interval.tick().await;
            if heartbeat_tx.send(P2PServerMessage::Pong).is_err() {
                break;
            }
        }
    });

    // Redis listener task — subscribes to Redis Pub/Sub channels for each
    // bet the client is watching and forwards messages to the broadcast channel.
    //
    // We use a dedicated Redis connection for Pub/Sub because a connection in
    // subscribe mode cannot issue regular commands.
    let redis_url = state.config().redis_url.clone();
    let registry_clone = state.p2p_registry().clone();
    let conn_id_clone = conn_id;
    let tx_clone = tx.clone();

    // Channel used to tell the Redis listener which bet channels to (un)subscribe.
    let (redis_cmd_tx, mut redis_cmd_rx) =
        tokio::sync::mpsc::unbounded_channel::<RedisListenerCmd>();

    let redis_listener_handle = tokio::spawn(async move {
        run_redis_listener(
            redis_url,
            registry_clone,
            conn_id_clone,
            tx_clone,
            &mut redis_cmd_rx,
        )
        .await;
    });

    // ── Main select loop ──────────────────────────────────────────────────────
    loop {
        tokio::select! {
            // Forward broadcast messages to the WebSocket client
            msg = rx.recv() => {
                match msg {
                    Ok(server_msg) => {
                        match serde_json::to_string(&server_msg) {
                            Ok(json) => {
                                if ws_sender.send(Message::Text(json)).await.is_err() {
                                    debug!("P2P WS client disconnected (conn_id={})", conn_id);
                                    break;
                                }
                            }
                            Err(e) => error!("Failed to serialize P2P WS message: {}", e),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("P2P WS subscriber lagged by {} messages (conn_id={})", n, conn_id);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("P2P WS broadcast channel closed (conn_id={})", conn_id);
                        break;
                    }
                }
            }

            // Handle messages from the WebSocket client
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_client_message(
                            &text,
                            conn_id,
                            &state,
                            &tx,
                            &redis_cmd_tx,
                        )
                        .await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!("P2P WS client sent close frame (conn_id={})", conn_id);
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Err(e)) => {
                        error!("P2P WS error (conn_id={}): {}", conn_id, e);
                        break;
                    }
                    None => {
                        debug!("P2P WS stream ended (conn_id={})", conn_id);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // ── Cleanup ───────────────────────────────────────────────────────────────
    heartbeat_handle.abort();
    redis_listener_handle.abort();

    // Unregister and get the set of bet IDs this connection was subscribed to
    let subscribed_bets = state.p2p_registry().unregister(conn_id).await;

    info!(
        "P2P WS closed: conn_id={}, user_id={}, was_subscribed_to={:?}",
        conn_id, user_id, subscribed_bets
    );
}

// ─── Client Message Handler ───────────────────────────────────────────────────

async fn handle_client_message(
    text: &str,
    conn_id: Uuid,
    state: &AppState,
    tx: &broadcast::Sender<P2PServerMessage>,
    redis_cmd_tx: &tokio::sync::mpsc::UnboundedSender<RedisListenerCmd>,
) {
    let msg = match serde_json::from_str::<P2PClientMessage>(text) {
        Ok(m) => m,
        Err(e) => {
            let err = P2PServerMessage::Error {
                message: format!("Invalid message format: {}", e),
            };
            let _ = tx.send(err);
            return;
        }
    };

    match msg {
        P2PClientMessage::SubscribeBet { bet_id } => {
            match state.p2p_registry().subscribe_bet(conn_id, bet_id).await {
                Ok(()) => {
                    // Tell the Redis listener to subscribe to this channel
                    let _ = redis_cmd_tx.send(RedisListenerCmd::Subscribe(bet_id));
                    let _ = tx.send(P2PServerMessage::Subscribed { bet_id });
                    info!("P2P WS conn {} subscribed to bet {}", conn_id, bet_id);
                }
                Err(e) => {
                    let _ = tx.send(P2PServerMessage::Error { message: e });
                }
            }
        }
        P2PClientMessage::UnsubscribeBet { bet_id } => {
            state.p2p_registry().unsubscribe_bet(conn_id, bet_id).await;
            // Tell the Redis listener to unsubscribe if no one else is watching
            if !state.p2p_registry().has_subscribers(bet_id).await {
                let _ = redis_cmd_tx.send(RedisListenerCmd::Unsubscribe(bet_id));
            }
            let _ = tx.send(P2PServerMessage::Unsubscribed { bet_id });
            info!("P2P WS conn {} unsubscribed from bet {}", conn_id, bet_id);
        }
        P2PClientMessage::Ping => {
            let _ = tx.send(P2PServerMessage::Pong);
        }
    }
}

// ─── Redis Listener ───────────────────────────────────────────────────────────

/// Commands sent to the Redis listener task.
enum RedisListenerCmd {
    Subscribe(i64),
    Unsubscribe(i64),
}

/// Runs in a dedicated Tokio task.  Maintains a Redis Pub/Sub connection and
/// dynamically subscribes/unsubscribes to bet channels as clients request.
/// When a message arrives on a channel it broadcasts it to all WebSocket
/// connections subscribed to that bet via the P2PConnectionRegistry.
///
/// Uses `redis::Client::get_async_connection().into_pubsub()` for a dedicated
/// Pub/Sub connection that is separate from the deadpool connection pool.
///
/// Because `PubSub::on_message()` borrows `&mut self` for the lifetime of the
/// stream, we use a two-task design:
///   - An inner "reader" task that owns the PubSub and forwards messages to an
///     mpsc channel.
///   - This outer task that processes subscribe/unsubscribe commands and
///     restarts the reader task whenever the subscription set changes.
async fn run_redis_listener(
    redis_url: String,
    registry: P2PConnectionRegistry,
    conn_id: Uuid,
    _tx: broadcast::Sender<P2PServerMessage>,
    cmd_rx: &mut tokio::sync::mpsc::UnboundedReceiver<RedisListenerCmd>,
) {
    let mut subscribed_channels: HashSet<String> = HashSet::new();

    // Channel for forwarding raw Redis messages from the reader task
    let (msg_tx, mut msg_rx) =
        tokio::sync::mpsc::unbounded_channel::<(String, String)>(); // (channel, payload)

    // Handle for the current reader task (None when no subscriptions)
    let mut reader_handle: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        tokio::select! {
            // Process subscribe/unsubscribe commands
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(RedisListenerCmd::Subscribe(bet_id)) => {
                        let channel = bet_channel(bet_id);
                        if subscribed_channels.insert(channel) {
                            // Restart the reader with the updated subscription set
                            restart_reader(
                                &redis_url,
                                &subscribed_channels,
                                &msg_tx,
                                &mut reader_handle,
                                conn_id,
                            ).await;
                        }
                    }
                    Some(RedisListenerCmd::Unsubscribe(bet_id)) => {
                        let channel = bet_channel(bet_id);
                        if subscribed_channels.remove(&channel) {
                            restart_reader(
                                &redis_url,
                                &subscribed_channels,
                                &msg_tx,
                                &mut reader_handle,
                                conn_id,
                            ).await;
                        }
                    }
                    None => {
                        debug!("Redis listener command channel closed (conn_id={})", conn_id);
                        break;
                    }
                }
            }

            // Forward messages from the reader task to subscribed WebSocket clients
            msg = msg_rx.recv() => {
                if let Some((channel, payload)) = msg {
                    if let Some(bet_id_str) = channel.strip_prefix("bet_updates:") {
                        if let Ok(bet_id) = bet_id_str.parse::<i64>() {
                            match serde_json::from_str::<BetUpdatePayload>(&payload) {
                                Ok(update) => {
                                    registry
                                        .broadcast_to_bet(bet_id, P2PServerMessage::BetUpdate(update))
                                        .await;
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to parse bet update payload for bet {}: {}",
                                        bet_id, e
                                    );
                                    if let Ok(data) =
                                        serde_json::from_str::<serde_json::Value>(&payload)
                                    {
                                        let fallback =
                                            P2PServerMessage::BetUpdate(BetUpdatePayload {
                                                bet_id,
                                                event_type: "update".to_string(),
                                                data,
                                                timestamp: chrono::Utc::now().timestamp(),
                                            });
                                        registry.broadcast_to_bet(bet_id, fallback).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Abort the reader task on exit
    if let Some(handle) = reader_handle {
        handle.abort();
    }
    debug!("Redis listener task exiting (conn_id={})", conn_id);
}

/// Aborts any existing reader task and starts a new one subscribed to
/// `channels`.  Does nothing if `channels` is empty.
///
/// Uses `redis::Client::get_async_pubsub()` (redis 0.24+) which returns a
/// dedicated `PubSub` connection.  After subscribing to all channels we call
/// `into_on_message()` to get an owned stream and forward messages to the
/// outer task via an mpsc channel.
async fn restart_reader(
    redis_url: &str,
    channels: &HashSet<String>,
    msg_tx: &tokio::sync::mpsc::UnboundedSender<(String, String)>,
    reader_handle: &mut Option<tokio::task::JoinHandle<()>>,
    conn_id: Uuid,
) {
    // Abort the previous reader
    if let Some(handle) = reader_handle.take() {
        handle.abort();
    }

    if channels.is_empty() {
        return;
    }

    let channels_vec: Vec<String> = channels.iter().cloned().collect();
    let url = redis_url.to_string();
    let tx = msg_tx.clone();

    let handle = tokio::spawn(async move {
        let client = match redis::Client::open(url.as_str()) {
            Ok(c) => c,
            Err(e) => {
                error!("Redis reader: failed to create client (conn_id={}): {}", conn_id, e);
                return;
            }
        };

        // get_tokio_connection().into_pubsub() is the correct API for redis 0.24
        let raw_conn = match client.get_tokio_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!("Redis reader: failed to get connection (conn_id={}): {}", conn_id, e);
                return;
            }
        };
        let mut pubsub = raw_conn.into_pubsub();

        for channel in &channels_vec {
            if let Err(e) = pubsub.subscribe(channel.as_str()).await {
                error!("Redis reader: failed to subscribe to {}: {}", channel, e);
                return;
            }
            debug!("Redis reader subscribed to {}", channel);
        }

        // into_on_message() consumes the PubSub and returns an owned stream,
        // allowing us to hold it without lifetime issues.
        let mut stream = pubsub.into_on_message();
        while let Some(redis_msg) = stream.next().await {
            let channel: String = match redis_msg.get_channel() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let payload: String = match redis_msg.get_payload() {
                Ok(p) => p,
                Err(_) => continue,
            };
            if tx.send((channel, payload)).is_err() {
                break; // Receiver dropped — outer task exited
            }
        }
        debug!("Redis reader stream ended (conn_id={})", conn_id);
    });

    *reader_handle = Some(handle);
}

// ─── Public Publishing Helper ─────────────────────────────────────────────────

/// Publish a bet update to Redis so all subscribed WebSocket clients receive it.
///
/// Call this from API route handlers whenever a bet's state changes.
///
/// ```rust
/// publish_bet_update(&state, 42, "participant_joined", serde_json::json!({
///     "user_id": user_id,
///     "position": "Yes",
///     "stake": 1000,
/// })).await;
/// ```
pub async fn publish_bet_update(
    state: &AppState,
    bet_id: i64,
    event_type: &str,
    data: serde_json::Value,
) {
    let payload = BetUpdatePayload {
        bet_id,
        event_type: event_type.to_string(),
        data,
        timestamp: chrono::Utc::now().timestamp(),
    };

    let json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to serialize bet update payload: {}", e);
            return;
        }
    };

    let channel = bet_channel(bet_id);

    match state.redis().get().await {
        Ok(mut conn) => {
            let result: Result<i64, _> = conn.publish(&channel, &json).await;
            match result {
                Ok(receivers) => {
                    debug!(
                        "Published bet update to {} (event={}, receivers={})",
                        channel, event_type, receivers
                    );
                }
                Err(e) => {
                    error!("Failed to publish to Redis channel {}: {}", channel, e);
                }
            }
        }
        Err(e) => {
            error!("Redis connection failed for publish: {}", e);
        }
    }
}

// ─── JWT Validation ───────────────────────────────────────────────────────────

fn validate_ws_token(token: &str, state: &AppState) -> Result<Claims, AppError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    let secret = state.config().jwt_secret.as_bytes();
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map_err(|e| AppError::Unauthorized(format!("Invalid P2P WS token: {}", e)))?;
    Ok(data.claims)
}
