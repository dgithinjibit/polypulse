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

pub mod p2p_bets;
pub use p2p_bets::{p2p_bets_ws_handler, P2PConnectionRegistry};

const CHANNEL_CAPACITY: usize = 64;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);
const MAX_CONNECTIONS_PER_USER: usize = 5;

// ─── Message Types ────────────────────────────────────────────────────────────

/// Client-to-server messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    SubscribePoll { poll_id: i64 },
    UnsubscribePoll { poll_id: i64 },
    Ping,
}

/// Server-to-client messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    PriceUpdate(PriceUpdateEvent),
    PollResolved(PollResolvedEvent),
    CommentAdded(CommentAddedEvent),
    Notification(NotificationEvent),
    ChallengeInvite(ChallengeInviteEvent),
    Pong,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdateEvent {
    pub poll_id: i64,
    pub prices: HashMap<String, f64>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollResolvedEvent {
    pub poll_id: i64,
    pub winning_option_id: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentAddedEvent {
    pub poll_id: i64,
    pub comment_id: i64,
    pub user_id: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub notification_id: i64,
    pub notification_type: String,
    pub message: String,
    pub actor_id: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeInviteEvent {
    pub challenge_id: i64,
    pub creator_id: String,
    pub amount: String,
    pub question: String,
    pub timestamp: i64,
}

// ─── Connection Registry ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ConnectionRegistry {
    /// Map of connection_id -> user_id
    connections: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    /// Map of user_id -> set of connection_ids
    user_connections: Arc<RwLock<HashMap<Uuid, HashSet<Uuid>>>>,
    /// Map of poll_id -> set of connection_ids
    poll_subscriptions: Arc<RwLock<HashMap<i64, HashSet<Uuid>>>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            poll_subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, conn_id: Uuid, user_id: Uuid) -> Result<(), String> {
        let mut user_conns = self.user_connections.write().await;
        let user_conn_set = user_conns.entry(user_id).or_insert_with(HashSet::new);
        
        if user_conn_set.len() >= MAX_CONNECTIONS_PER_USER {
            return Err(format!("Max connections ({}) reached for user", MAX_CONNECTIONS_PER_USER));
        }
        
        user_conn_set.insert(conn_id);
        drop(user_conns);

        let mut conns = self.connections.write().await;
        conns.insert(conn_id, user_id);
        
        Ok(())
    }

    pub async fn unregister(&self, conn_id: Uuid) {
        let mut conns = self.connections.write().await;
        if let Some(user_id) = conns.remove(&conn_id) {
            drop(conns);
            
            let mut user_conns = self.user_connections.write().await;
            if let Some(set) = user_conns.get_mut(&user_id) {
                set.remove(&conn_id);
                if set.is_empty() {
                    user_conns.remove(&user_id);
                }
            }
            drop(user_conns);

            // Remove from all poll subscriptions
            let mut poll_subs = self.poll_subscriptions.write().await;
            for set in poll_subs.values_mut() {
                set.remove(&conn_id);
            }
        }
    }

    pub async fn subscribe_poll(&self, conn_id: Uuid, poll_id: i64) {
        let mut poll_subs = self.poll_subscriptions.write().await;
        poll_subs.entry(poll_id).or_insert_with(HashSet::new).insert(conn_id);
    }

    pub async fn unsubscribe_poll(&self, conn_id: Uuid, poll_id: i64) {
        let mut poll_subs = self.poll_subscriptions.write().await;
        if let Some(set) = poll_subs.get_mut(&poll_id) {
            set.remove(&conn_id);
            if set.is_empty() {
                poll_subs.remove(&poll_id);
            }
        }
    }

    pub async fn get_poll_subscribers(&self, poll_id: i64) -> Vec<Uuid> {
        let poll_subs = self.poll_subscriptions.read().await;
        poll_subs.get(&poll_id).map(|set| set.iter().copied().collect()).unwrap_or_default()
    }

    pub async fn get_user_connections(&self, user_id: Uuid) -> Vec<Uuid> {
        let user_conns = self.user_connections.read().await;
        user_conns.get(&user_id).map(|set| set.iter().copied().collect()).unwrap_or_default()
    }
}

// ─── Broadcast Hub ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BroadcastHub {
    registry: ConnectionRegistry,
    /// Per-connection broadcast channels
    senders: Arc<RwLock<HashMap<Uuid, broadcast::Sender<ServerMessage>>>>,
}

impl BroadcastHub {
    pub fn new(registry: ConnectionRegistry) -> Self {
        Self {
            registry,
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_sender(&self, conn_id: Uuid, sender: broadcast::Sender<ServerMessage>) {
        let mut senders = self.senders.write().await;
        senders.insert(conn_id, sender);
    }

    pub async fn unregister_sender(&self, conn_id: Uuid) {
        let mut senders = self.senders.write().await;
        senders.remove(&conn_id);
    }

    pub async fn broadcast_to_poll(&self, poll_id: i64, message: ServerMessage) {
        let conn_ids = self.registry.get_poll_subscribers(poll_id).await;
        let senders = self.senders.read().await;
        
        for conn_id in conn_ids {
            if let Some(tx) = senders.get(&conn_id) {
                let _ = tx.send(message.clone());
            }
        }
    }

    pub async fn send_to_user(&self, user_id: Uuid, message: ServerMessage) {
        let conn_ids = self.registry.get_user_connections(user_id).await;
        let senders = self.senders.read().await;
        
        for conn_id in conn_ids {
            if let Some(tx) = senders.get(&conn_id) {
                let _ = tx.send(message.clone());
            }
        }
    }
}

/// Market Hub (Legacy compatibility)
#[derive(Clone)]
pub struct MarketHub {
    #[allow(dead_code)]
    broadcast_hub: BroadcastHub,
}

impl MarketHub {
    pub fn new(broadcast_hub: BroadcastHub) -> Self {
        Self { broadcast_hub }
    }

    #[allow(dead_code)]
    pub fn broadcast(&self, update: PriceUpdateEvent) {
        let hub = self.broadcast_hub.clone();
        let poll_id = update.poll_id;
        tokio::spawn(async move {
            hub.broadcast_to_poll(poll_id, ServerMessage::PriceUpdate(update)).await;
        });
    }
}

// ─── WebSocket Query Params ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: String,
}

// ─── WebSocket Handler ────────────────────────────────────────────────────────

/// GET /ws?token=<jwt>
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Validate JWT from query parameter
    let claims = match validate_ws_token(&query.token, &state) {
        Ok(c) => c,
        Err(e) => {
            warn!("WS auth failed: {}", e);
            return axum::http::Response::builder()
                .status(401)
                .body(axum::body::Body::from("Invalid or missing token"))
                .unwrap()
                .into_response();
        }
    };

    let user_id = claims.sub;
    info!("WS connection established: user={}", user_id);

    ws.on_upgrade(move |socket| handle_websocket(socket, user_id, state))
}

async fn handle_websocket(socket: WebSocket, user_id: Uuid, state: AppState) {
    let conn_id = Uuid::new_v4();
    
    // Register connection
    if let Err(e) = state.connection_registry().register(conn_id, user_id).await {
        error!("Failed to register WS connection: {}", e);
        return;
    }

    // Store connection in Redis
    if let Err(e) = store_connection_in_redis(&state, conn_id, user_id).await {
        error!("Failed to store connection in Redis: {}", e);
        state.connection_registry().unregister(conn_id).await;
        return;
    }

    // Create broadcast channel for this connection
    let (tx, _) = broadcast::channel::<ServerMessage>(CHANNEL_CAPACITY);
    state.broadcast_hub().register_sender(conn_id, tx.clone()).await;

    info!("WS connection registered: conn_id={}, user_id={}", conn_id, user_id);

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Create receiver for broadcast messages
    let mut rx = tx.subscribe();

    // Heartbeat task
    let heartbeat_tx = tx.clone();
    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        loop {
            interval.tick().await;
            if heartbeat_tx.send(ServerMessage::Pong).is_err() {
                break;
            }
        }
    });

    // Main message loop
    loop {
        tokio::select! {
            // Receive broadcast messages and send to client
            msg = rx.recv() => {
                match msg {
                    Ok(server_msg) => {
                        let json = match serde_json::to_string(&server_msg) {
                            Ok(j) => j,
                            Err(e) => {
                                error!("Failed to serialize message: {}", e);
                                continue;
                            }
                        };
                        if sender.send(Message::Text(json)).await.is_err() {
                            debug!("Client disconnected (conn_id={})", conn_id);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WS subscriber lagged by {} messages (conn_id={})", n, conn_id);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("Broadcast channel closed (conn_id={})", conn_id);
                        break;
                    }
                }
            }
            // Receive messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_client_message(&text, conn_id, &state).await {
                            error!("Error handling client message: {}", e);
                            let error_msg = ServerMessage::Error {
                                message: e.to_string(),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                let _ = sender.send(Message::Text(json)).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        debug!("Client sent close frame: {:?}", frame);
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Client responded to ping
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        debug!("Client disconnected (conn_id={})", conn_id);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    heartbeat_handle.abort();
    state.broadcast_hub().unregister_sender(conn_id).await;
    state.connection_registry().unregister(conn_id).await;
    if let Err(e) = remove_connection_from_redis(&state, conn_id).await {
        error!("Failed to remove connection from Redis: {}", e);
    }
    
    info!("WS connection closed: conn_id={}, user_id={}", conn_id, user_id);
}

async fn handle_client_message(
    text: &str,
    conn_id: Uuid,
    state: &AppState,
) -> Result<(), AppError> {
    let msg: ClientMessage = serde_json::from_str(text)
        .map_err(|e| AppError::BadRequest(format!("Invalid message format: {}", e)))?;

    match msg {
        ClientMessage::SubscribePoll { poll_id } => {
            state.connection_registry().subscribe_poll(conn_id, poll_id).await;
            info!("Connection {} subscribed to poll {}", conn_id, poll_id);
        }
        ClientMessage::UnsubscribePoll { poll_id } => {
            state.connection_registry().unsubscribe_poll(conn_id, poll_id).await;
            info!("Connection {} unsubscribed from poll {}", conn_id, poll_id);
        }
        ClientMessage::Ping => {
            // Heartbeat handled automatically
        }
    }

    Ok(())
}

async fn store_connection_in_redis(
    state: &AppState,
    conn_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let mut conn = state.redis().get().await
        .map_err(|e| AppError::InternalServerError(format!("Redis connection failed: {}", e)))?;
    
    let key = format!("ws:conn:{}", conn_id);
    let _: () = conn.set_ex(&key, user_id.to_string(), 3600).await
        .map_err(|e| AppError::InternalServerError(format!("Redis set failed: {}", e)))?;
    
    Ok(())
}

async fn remove_connection_from_redis(
    state: &AppState,
    conn_id: Uuid,
) -> Result<(), AppError> {
    let mut conn = state.redis().get().await
        .map_err(|e| AppError::InternalServerError(format!("Redis connection failed: {}", e)))?;
    
    let key = format!("ws:conn:{}", conn_id);
    let _: () = conn.del(&key).await
        .map_err(|e| AppError::InternalServerError(format!("Redis del failed: {}", e)))?;
    
    Ok(())
}

fn validate_ws_token(token: &str, state: &AppState) -> Result<Claims, AppError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    let secret = state.config().jwt_secret.as_bytes();
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &Validation::default())
        .map_err(|e| AppError::Unauthorized(format!("Invalid WS token: {}", e)))?;
    Ok(data.claims)
}

// ─── Public Broadcasting Functions ────────────────────────────────────────────

pub async fn broadcast_price_update(state: &AppState, poll_id: i64, prices: HashMap<String, f64>) {
    let event = PriceUpdateEvent {
        poll_id,
        prices,
        timestamp: chrono::Utc::now().timestamp(),
    };
    state.broadcast_hub().broadcast_to_poll(poll_id, ServerMessage::PriceUpdate(event)).await;
}

pub async fn broadcast_poll_resolved(state: &AppState, poll_id: i64, winning_option_id: i64) {
    let event = PollResolvedEvent {
        poll_id,
        winning_option_id,
        timestamp: chrono::Utc::now().timestamp(),
    };
    state.broadcast_hub().broadcast_to_poll(poll_id, ServerMessage::PollResolved(event)).await;
}

pub async fn broadcast_comment_added(
    state: &AppState,
    poll_id: i64,
    comment_id: i64,
    user_id: Uuid,
    content: String,
) {
    let event = CommentAddedEvent {
        poll_id,
        comment_id,
        user_id: user_id.to_string(),
        content,
        timestamp: chrono::Utc::now().timestamp(),
    };
    state.broadcast_hub().broadcast_to_poll(poll_id, ServerMessage::CommentAdded(event)).await;
}

pub async fn send_notification_to_user(
    state: &AppState,
    user_id: Uuid,
    notification_id: i64,
    notification_type: String,
    message: String,
    actor_id: Option<Uuid>,
) {
    let event = NotificationEvent {
        notification_id,
        notification_type,
        message,
        actor_id: actor_id.map(|id| id.to_string()),
        timestamp: chrono::Utc::now().timestamp(),
    };
    state.broadcast_hub().send_to_user(user_id, ServerMessage::Notification(event)).await;
}

pub async fn send_challenge_invite(
    state: &AppState,
    user_id: Uuid,
    challenge_id: i64,
    creator_id: Uuid,
    amount: String,
    question: String,
) {
    let event = ChallengeInviteEvent {
        challenge_id,
        creator_id: creator_id.to_string(),
        amount,
        question,
        timestamp: chrono::Utc::now().timestamp(),
    };
    state.broadcast_hub().send_to_user(user_id, ServerMessage::ChallengeInvite(event)).await;
}
