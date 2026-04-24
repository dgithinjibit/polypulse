// ============================================================
// FILE: main.rs
// PURPOSE: The entry point of the PolyPulse Rust backend server.
//          This file:
//            1. Loads environment variables from .env file
//            2. Initializes structured JSON logging (tracing)
//            3. Loads all configuration from environment variables
//            4. Creates the shared application state (DB pool, Redis pool)
//            5. Builds the Axum HTTP router with all routes
//            6. Spawns a background task for auto-closing expired polls
//            7. Starts the TCP listener and serves the application
//
// FRAMEWORK: Axum (async Rust web framework built on Tokio + Hyper)
// ASYNC RUNTIME: Tokio (the most popular async runtime for Rust)
//
// JUNIOR DEV NOTE:
//   In Rust, the main function is the program entry point.
//   #[tokio::main] transforms it into an async function that runs on Tokio's runtime.
//   anyhow::Result<()> means the function can return any error type (convenient for main).
// ============================================================

// Allow dead code warnings to be suppressed across the whole crate.
// Some structs/functions may be defined but not yet used - this prevents noisy warnings.
#![allow(dead_code)]

// ---- MODULE DECLARATIONS ----
// These tell Rust to include the code from each corresponding file.
// Each mod corresponds to a file: config.rs, db.rs, etc.

mod config;     // Configuration loading from environment variables
mod db;         // Database connection pool setup
mod errors;     // Custom error types and HTTP error responses
mod lmsr;       // Logarithmic Market Scoring Rule (prediction market pricing algorithm)
mod middleware; // HTTP middleware (auth, rate limiting, request IDs, validation)
mod models;     // Database model structs (User, Market, Bet, etc.)
mod routes;     // HTTP route handlers organized by domain
mod services;   // Business logic services (cache, M-Pesa, wallet, etc.)
mod state;      // Shared application state (AppState struct)
mod ws;         // WebSocket handling for real-time updates

// ---- STANDARD LIBRARY IMPORTS ----
// SocketAddr: represents a network address (IP + port) e.g., "0.0.0.0:8000"
use std::net::SocketAddr;

// ---- TRACING IMPORTS ----
// info!: macro for logging informational messages (structured JSON in production)
use tracing::info;

// tracing_subscriber: configures how log messages are collected and formatted
// SubscriberExt: trait for composing subscriber layers
// SubscriberInitExt: trait for initializing the global subscriber
// EnvFilter: reads log level from RUST_LOG environment variable
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ---- LOCAL IMPORTS ----
// Config: our configuration struct loaded from environment variables
// AppState: the shared state passed to every HTTP handler
use crate::{config::Config, state::AppState};

// ============================================================
// FUNCTION: main
// PURPOSE: Application entry point. Sets up everything and starts the server.
// RETURNS: anyhow::Result<()> - Ok(()) on clean shutdown, Err on startup failure
// The ? operator propagates errors up - if any setup step fails, the program exits.
// ============================================================
#[tokio::main]  // This macro sets up the Tokio async runtime and calls main() on it
async fn main() -> anyhow::Result<()> {
    // STEP 1: Load .env file
    // dotenvy reads key=value pairs from .env and sets them as environment variables.
    // The let _ = ... ignores the result - it's non-fatal if .env doesn't exist
    // (e.g., in production where env vars are set directly in the environment).
    let _ = dotenvy::dotenv();

    // STEP 2: Initialize structured logging
    // We use the `tracing` ecosystem for structured, async-aware logging.
    // .with(EnvFilter): reads RUST_LOG env var to set log levels per module.
    //   e.g., RUST_LOG=backend=debug,tower_http=info
    //   Falls back to "backend=debug,tower_http=debug" if RUST_LOG is not set.
    // .with(fmt::layer().json()): formats log output as JSON (great for log aggregators)
    // .init(): sets this as the global subscriber (must be called once)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "backend=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Log that we're starting up - this will appear in the server logs
    info!("Starting PolyPulse Rust backend");

    // STEP 3: Load configuration from environment variables
    // Config::from_env() reads all required env vars and returns a Config struct.
    // The ? propagates any error (e.g., missing DATABASE_URL) and exits the program.
    let config = Config::from_env()?;

    // STEP 4: Build shared application state
    // AppState::new() creates:
    //   - PostgreSQL connection pool (connects to DATABASE_URL)
    //   - Redis connection pool (connects to REDIS_URL)
    //   - WebSocket hubs for real-time communication
    // The state is wrapped in Arc internally so it can be cheaply cloned and shared
    // across all request handlers.
    let state = AppState::new(config).await?;

    // STEP 5: Build the Axum router
    // routes::build_router() creates the full HTTP router with all endpoints,
    // middleware (CORS, auth, rate limiting), and injects the shared state.
    let app = routes::build_router(state.clone());

    // STEP 6: Spawn background task for auto-closing polls
    // Some prediction markets have an expiry time. This background task runs
    // continuously and closes any markets that have passed their close time.
    // tokio::spawn() runs this as a separate async task (like a background thread).
    // We clone the DB pool so the background task has its own reference.
    let db_clone = state.db().clone();
    tokio::spawn(async move {
        // This runs forever in the background, periodically checking for expired polls
        services::poll_closer::poll_auto_closer(db_clone).await;
    });

    // STEP 6.1: Spawn background task for notification cleanup (30-day retention)
    let db_clone_notifications = state.db().clone();
    tokio::spawn(async move {
        loop {
            // Run cleanup once per day
            tokio::time::sleep(tokio::time::Duration::from_secs(86400)).await;
            match services::p2p_notifications::cleanup_old_notifications(&db_clone_notifications).await {
                Ok(count) => info!("Cleaned up {} old notifications", count),
                Err(e) => tracing::error!("Failed to cleanup notifications: {}", e),
            }
        }
    });

    // STEP 6.2: Spawn background task for checking bets ending soon and ended bets
    let state_clone_bets = state.clone();
    tokio::spawn(async move {
        loop {
            // Check every 5 minutes
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            
            // Check for bets ending soon (1 hour before)
            if let Err(e) = services::p2p_notifications::check_bets_ending_soon(&state_clone_bets).await {
                tracing::error!("Failed to check bets ending soon: {}", e);
            }
            
            // Check for ended bets
            if let Err(e) = services::p2p_notifications::check_ended_bets(&state_clone_bets).await {
                tracing::error!("Failed to check ended bets: {}", e);
            }
        }
    });

    // STEP 7: Parse the server address
    // Bind to all network interfaces (0.0.0.0) on the configured port.
    // 0.0.0.0 means "accept connections on any network interface" (not just localhost).
    let addr: SocketAddr = format!("0.0.0.0:{}", state.config().port).parse()?;
    info!("Listening on {addr}");

    // STEP 8: Create the TCP listener and start serving
    // TcpListener::bind() opens the port and starts listening for connections.
    // axum::serve() runs the server indefinitely, handling requests until the process exits.
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    // If we reach here, the server shut down cleanly
    Ok(())
} // end main
