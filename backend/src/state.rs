// ============================================================
// FILE: state.rs
// PURPOSE: Defines the shared application state (AppState) that is
//          injected into every Axum HTTP handler.
//          Contains:
//            - PostgreSQL connection pool (for database queries)
//            - Redis connection pool (for caching and sessions)
//            - Application configuration
//            - WebSocket hubs (for real-time market updates)
//
// PATTERN: Arc<Inner> (Atomically Reference Counted)
//   AppState is cloned cheaply for every request handler.
//   The actual data lives in Arc<Inner> - cloning AppState just increments
//   the reference count, not the data itself.
//   This is the standard Rust pattern for shared state in async web servers.
//
// USAGE in handlers:
//   async fn my_handler(State(state): State<AppState>) -> impl IntoResponse {
//       let user = sqlx::query!("SELECT * FROM users").fetch_one(state.db()).await?;
//   }
//
// JUNIOR DEV NOTE:
//   In Rust, you can't share mutable state between threads without synchronization.
//   Arc (Atomic Reference Count) allows multiple owners of the same data.
//   The data inside Arc is immutable - connection pools handle their own internal mutability.
// ============================================================

// Arc: thread-safe reference counting for shared ownership
use std::sync::Arc;

// Result: anyhow's flexible error type for the constructor
use anyhow::Result;

// deadpool_redis: async Redis connection pool
// Config as RedisConfig: Redis pool configuration
// Pool as RedisPool: the Redis connection pool type
// Runtime: specifies which async runtime to use (Tokio)
use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};

// PgPool: PostgreSQL connection pool from sqlx
use sqlx::PgPool;

// info!: structured logging macro
use tracing::info;

// Local imports
use crate::{
    config::Config,  // Our configuration struct
    db,              // Database pool creation function
    ws::{
        BroadcastHub,           // Hub for broadcasting messages to WebSocket clients
        ConnectionRegistry,     // Registry tracking active WebSocket connections
        MarketHub,              // Hub for market-specific WebSocket channels
        P2PConnectionRegistry,  // Registry for P2P bet WebSocket subscriptions
    }
};

// ============================================================
// STRUCT: AppState
// PURPOSE: The top-level shared state passed to every Axum handler.
//          Implements Clone cheaply because Arc<Inner> clone is just a pointer copy.
//          The #[derive(Clone)] works because all fields implement Clone.
//
// AXUM INTEGRATION:
//   Registered with the router via .with_state(state)
//   Extracted in handlers via State<AppState> extractor
// ============================================================
#[derive(Clone)]
pub struct AppState {
    /// The core application data (DB, Redis, Config) wrapped in Arc for cheap cloning.
    /// Access via state.db(), state.redis(), state.config() helper methods.
    pub inner: Arc<Inner>,

    /// Hub for market-specific WebSocket channels.
    /// Allows broadcasting price updates to users watching a specific market.
    /// #[allow(dead_code)] suppresses warning if not yet used everywhere.
    #[allow(dead_code)]
    pub market_hub: MarketHub,

    /// Registry of all active WebSocket connections.
    /// Used to track which users are connected and send targeted messages.
    pub connection_registry: ConnectionRegistry,

    /// Hub for broadcasting messages to all connected WebSocket clients.
    /// Used for system-wide notifications and real-time updates.
    pub broadcast_hub: BroadcastHub,

    /// Registry for P2P bet WebSocket subscriptions.
    /// Tracks which connections are subscribed to which bet IDs and
    /// provides broadcasting to bet-specific subscriber sets.
    pub p2p_registry: P2PConnectionRegistry,
} // end AppState struct

// ============================================================
// STRUCT: Inner
// PURPOSE: The actual data held inside Arc<Inner>.
//          Separated from AppState so we can wrap it in Arc without
//          wrapping the WebSocket hubs (which have their own Arc internally).
// ============================================================
pub struct Inner {
    /// PostgreSQL connection pool.
    /// sqlx manages a pool of connections for concurrent database access.
    /// Use state.db() to access this.
    pub db: PgPool,

    /// Redis connection pool.
    /// deadpool_redis manages async Redis connections.
    /// Use state.redis() to access this.
    pub redis: RedisPool,

    /// Application configuration loaded from environment variables.
    /// Use state.config() to access this.
    pub config: Config,
} // end Inner struct

// ============================================================
// IMPLEMENTATION: AppState
// ============================================================
impl AppState {
    // ============================================================
    // FUNCTION: new
    // PURPOSE: Creates a new AppState by connecting to all external services.
    //          Called once at startup in main.rs.
    // PARAM config: The loaded configuration (database URL, Redis URL, etc.)
    // RETURNS: Result<AppState> - Ok if all connections succeed, Err otherwise
    // ============================================================
    pub async fn new(config: Config) -> Result<Self> {
        // Connect to PostgreSQL using the database URL from config.
        // db::create_pool() creates a connection pool and runs a test query.
        // The ? propagates any connection error up to main().
        let db = db::create_pool(&config.database_url).await?;

        // Connect to Redis
        info!("Connecting to Redis at {}", config.redis_url);

        // Create Redis pool configuration from the URL
        let redis_cfg = RedisConfig::from_url(&config.redis_url);

        // Create the Redis connection pool using Tokio as the async runtime
        // Runtime::Tokio1 tells deadpool to use Tokio for async operations
        let redis = redis_cfg.create_pool(Some(Runtime::Tokio1))?;
        info!("Redis pool established");

        // Create WebSocket infrastructure
        // ConnectionRegistry: tracks all active WebSocket connections
        let connection_registry = ConnectionRegistry::new();

        // BroadcastHub: broadcasts messages to all connected clients
        // Takes a clone of the registry so it can look up connections
        let broadcast_hub = BroadcastHub::new(connection_registry.clone());

        // MarketHub: market-specific WebSocket channels
        // Takes a clone of the broadcast hub to send messages
        let market_hub = MarketHub::new(broadcast_hub.clone());

        // P2PConnectionRegistry: tracks P2P bet WebSocket subscriptions
        let p2p_registry = P2PConnectionRegistry::new();

        // Assemble and return the complete AppState
        Ok(Self {
            // Wrap the core data in Arc for cheap cloning across request handlers
            inner: Arc::new(Inner { db, redis, config }),
            market_hub,
            connection_registry,
            broadcast_hub,
            p2p_registry,
        })
    } // end new

    // ============================================================
    // ACCESSOR: db
    // PURPOSE: Returns a reference to the PostgreSQL connection pool.
    //          Convenience method so handlers can write state.db() instead of
    //          state.inner.db
    // ============================================================
    pub fn db(&self) -> &PgPool {
        &self.inner.db
    } // end db

    // ============================================================
    // ACCESSOR: redis
    // PURPOSE: Returns a reference to the Redis connection pool.
    //          Convenience method for state.inner.redis
    // ============================================================
    pub fn redis(&self) -> &RedisPool {
        &self.inner.redis
    } // end redis

    // ============================================================
    // ACCESSOR: config
    // PURPOSE: Returns a reference to the application configuration.
    //          Convenience method for state.inner.config
    // ============================================================
    pub fn config(&self) -> &Config {
        &self.inner.config
    } // end config

    // ============================================================
    // ACCESSOR: connection_registry
    // PURPOSE: Returns a reference to the WebSocket connection registry.
    //          Used by WebSocket handlers to register/unregister connections.
    // ============================================================
    pub fn connection_registry(&self) -> &ConnectionRegistry {
        &self.connection_registry
    } // end connection_registry

    // ============================================================
    // ACCESSOR: broadcast_hub
    // PURPOSE: Returns a reference to the WebSocket broadcast hub.
    //          Used by route handlers to send real-time updates to clients.
    // ============================================================
    pub fn broadcast_hub(&self) -> &BroadcastHub {
        &self.broadcast_hub
    } // end broadcast_hub

    // ============================================================
    // ACCESSOR: p2p_registry
    // PURPOSE: Returns a reference to the P2P bet WebSocket registry.
    //          Used by the P2P bets WebSocket handler and API routes
    //          to manage subscriptions and broadcast bet updates.
    // ============================================================
    pub fn p2p_registry(&self) -> &P2PConnectionRegistry {
        &self.p2p_registry
    } // end p2p_registry
} // end impl AppState
