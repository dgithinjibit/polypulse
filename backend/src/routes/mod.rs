// ============================================================
// FILE: routes/mod.rs
// PURPOSE: Assembles the complete Axum HTTP router for the PolyPulse backend.
//          This is the "routing table" of the application - it maps every
//          HTTP endpoint to its handler function and applies middleware.
//
// ROUTE GROUPS (by rate limiting tier):
//   1. auth_routes    - Authentication endpoints (10 req/min - strictest)
//   2. trading_routes - Betting/trading endpoints (30 req/min)
//   3. public         - Public read-only endpoints (general rate limit)
//   4. websocket      - WebSocket connection endpoint (no rate limit)
//   5. protected      - Authenticated user endpoints (JWT required + general rate limit)
//
// MIDDLEWARE STACK (applied to all routes, outermost to innermost):
//   1. request_id     - Adds a unique X-Request-ID header to every request
//   2. cors           - Handles Cross-Origin Resource Sharing (browser security)
//   3. compression    - Gzip/brotli response compression
//   4. trace          - HTTP request/response logging
//
// JUNIOR DEV NOTE:
//   Axum middleware is applied in reverse order - the last .layer() runs first.
//   Think of it like wrapping a gift: the outermost layer is applied last but
//   encountered first when unwrapping.
// ============================================================

// Declare all route handler modules
// Each pub mod corresponds to a file in the routes/ directory
pub mod auth;           // Authentication: login, register, wallet auth, token refresh
pub mod bets;           // Betting: place bets, sell shares, get positions
pub mod categories;     // Categories: list poll categories
pub mod challenges;     // Challenges: create, accept, resolve, cancel challenges
pub mod comments;       // Comments: list, create, like/unlike comments on polls
pub mod health;         // Health check: simple endpoint to verify server is running
pub mod markets;        // Markets: get prices and price history for prediction markets
pub mod notifications;  // Notifications: list, mark read, get unread count
pub mod p2p_bets;       // P2P Bets: create, join, report outcome, confirm outcome
pub mod paymaster;      // Paymaster: gasless transaction relay for blockchain ops
pub mod polls;          // Polls: create, list, resolve, suspend, cancel prediction markets
pub mod users;          // Users: get current user profile and portfolio
pub mod wagers;         // Wagers: create, list, accept, cancel peer-to-peer wagers
pub mod wallet;         // Wallet: get balance, transaction history

// Axum imports for building the router
use axum::{
    middleware,                  // For applying middleware functions to route groups
    routing::{get, post},        // HTTP method routing helpers
    Router,                      // The main router type
};

// Tower: middleware composition library
use tower::ServiceBuilder;

// Tower-HTTP: HTTP-specific middleware
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

// HTTP types for security headers
use axum::http::{header, HeaderValue};

// Local imports
use crate::{
    middleware as mw,  // Our custom middleware (auth, rate_limit, request_id, validation)
    state::AppState,   // Shared application state
    ws,                // WebSocket handler
};

// ============================================================
// FUNCTION: build_router
// PURPOSE: Creates and returns the complete Axum router with all routes,
//          middleware, and shared state.
//          Called once in main.rs during startup.
// PARAM state: The shared application state (DB pool, Redis, config)
// RETURNS: Router - the fully configured Axum router ready to serve requests
// ============================================================
pub fn build_router(state: AppState) -> Router {
    // ---- CORS CONFIGURATION ----
    // Parse allowed origins from config (CORS_ORIGINS env var, comma-separated).
    // Falls back to permissive Any only if no origins are configured (dev mode).
    let cors = {
        use tower_http::cors::{AllowOrigin, Any};
        use axum::http::HeaderValue;

        let origins: Vec<HeaderValue> = state
            .config()
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();

        if origins.is_empty() {
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            // Use a predicate so we can match both exact origins (from CORS_ORIGINS env var)
            // and wildcard patterns like *.ipfs.4everland.app (which change hash on every deploy).
            let origins_clone = origins.clone();
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
                    // Allow any exact match from the configured list
                    if origins_clone.contains(origin) {
                        return true;
                    }
                    // Allow any 4everland IPFS deployment subdomain
                    origin
                        .to_str()
                        .map(|s| s.ends_with(".ipfs.4everland.app"))
                        .unwrap_or(false)
                }))
                .allow_methods(Any)
                .allow_headers(Any)
        }
    };

    // ---- AUTH ROUTES (10 req/min rate limit) ----
    // Authentication endpoints have the strictest rate limiting to prevent
    // brute force attacks on login and credential stuffing.
    let auth_routes = Router::new()
        // POST /api/v1/auth/login - Login with username/password
        .route("/api/v1/auth/login", post(auth::login))

        // POST /api/v1/auth/refresh - Exchange refresh token for new access token
        .route("/api/v1/auth/refresh", post(auth::token_refresh))

        // POST /api/v1/auth/register - Create a new account
        .route("/api/v1/auth/register", post(auth::register))

        // GET /api/v1/auth/verify-email/:token - Verify email address via link
        .route("/api/v1/auth/verify-email/:token", get(auth::verify_email))

        // POST /api/v1/auth/stellar-nonce - Get a nonce for Stellar wallet authentication
        // Step 1 of the Stellar wallet login flow
        .route("/api/v1/auth/stellar-nonce", post(auth::stellar_nonce))

        // POST /api/v1/auth/stellar-login - Authenticate with signed Stellar transaction
        // Step 2 of the Stellar wallet login flow - verifies the signature
        .route("/api/v1/auth/stellar-login", post(auth::stellar_login))

        // POST /api/v1/auth/near-nonce - Get a nonce for NEAR wallet authentication
        .route("/api/v1/auth/near-nonce", post(auth::near_nonce))

        // POST /api/v1/auth/near-login - Authenticate with NEAR wallet signature
        .route("/api/v1/auth/near-login", post(auth::near_login))

        // POST /api/v1/auth/omnichain-nonce - Get a nonce for any chain authentication
        .route("/api/v1/auth/omnichain-nonce", post(auth::omnichain_nonce))

        // POST /api/v1/auth/omnichain-login - Authenticate with any chain wallet signature
        .route("/api/v1/auth/omnichain-login", post(auth::omnichain_login))

        // Apply auth-specific rate limiting (10 requests per minute per IP)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::rate_limit::rate_limit_auth,
        ));

    // ---- TRADING ROUTES (30 req/min rate limit) ----
    // Trading endpoints have moderate rate limiting - users need to trade
    // frequently but we still want to prevent abuse.
    let trading_routes = Router::new()
        // POST /api/v1/bets - Place a bet on a prediction market
        .route("/api/v1/bets", post(bets::place_bet))

        // POST /api/v1/bets/sell - Sell shares in a prediction market
        .route("/api/v1/bets/sell", post(bets::sell_shares))

        // GET /api/v1/markets/:poll_id/prices - Get current prices for a market
        .route("/api/v1/markets/:poll_id/prices", get(markets::get_market_prices))

        // GET /api/v1/markets/:poll_id/history - Get price history for a market
        .route("/api/v1/markets/:poll_id/history", get(markets::get_price_history))

        // Apply trading-specific rate limiting (30 requests per minute per IP)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::rate_limit::rate_limit_trading,
        ));

    // ---- M-PESA ROUTES (commented out - using Freighter wallet instead) ----
    // M-Pesa is Kenya's mobile money service. These routes were for fiat deposits.
    // Currently disabled as the app uses Stellar/Freighter for all transactions.
    // let mpesa_routes = Router::new()
    //     .route("/api/v1/wallet/mpesa/deposit", post(wallet::initiate_mpesa_deposit))
    //     .route("/api/v1/wallet/mpesa/status/:checkout_id", get(wallet::get_mpesa_status))
    //     .layer(middleware::from_fn_with_state(
    //         state.clone(),
    //         mw::rate_limit::rate_limit_mpesa,  // 5 req/min - strictest
    //     ));

    // ---- PUBLIC ROUTES (general rate limit, no auth required) ----
    // These endpoints are accessible without authentication.
    // Anyone can browse markets, challenges, and wagers.
    let public = Router::new()
        // GET /health - Simple health check (returns 200 OK if server is running)
        .route("/health", get(health::health_check))

        // GET /api/v1/categories - List all poll categories
        .route("/api/v1/categories", get(categories::list_categories))

        // GET /api/v1/polls - List all prediction markets/polls
        .route("/api/v1/polls", get(polls::list_polls))

        // GET /api/v1/polls/:id - Get details of a specific poll
        .route("/api/v1/polls/:id", get(polls::get_poll_detail))

        // GET /api/v1/polls/:poll_id/comments - List comments on a poll
        .route("/api/v1/polls/:poll_id/comments", get(comments::list_comments))

        // GET /api/v1/challenges - List all challenges
        .route("/api/v1/challenges", get(challenges::list_challenges))

        // GET /api/v1/challenges/:id - Get details of a specific challenge
        .route("/api/v1/challenges/:id", get(challenges::get_challenge_detail))

        // GET /api/v1/wagers/:id - Get a wager by its share link
        .route("/api/v1/wagers/:id", get(wagers::get_wager_by_link))

        // Apply general rate limiting
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::rate_limit::rate_limit_general,
        ));

    // ---- WEBSOCKET ROUTE (no rate limiting on connection) ----
    // WebSocket connections are long-lived - rate limiting doesn't apply the same way.
    // Auth is handled via query parameter in the WebSocket handshake.
    let websocket = Router::new()
        // GET /ws - WebSocket upgrade endpoint for real-time updates
        .route("/ws", get(ws::ws_handler));

    // ---- PROTECTED ROUTES (JWT required + general rate limit) ----
    // These endpoints require a valid JWT token (enforced by require_auth middleware).
    // The auth middleware is applied first, then rate limiting.
    let protected = Router::new()
        // POST /api/v1/auth/logout - Invalidate the current session
        .route("/api/v1/auth/logout", post(auth::logout))

        // POST /api/v1/polls - Create a new prediction market
        .route("/api/v1/polls", post(polls::create_poll))

        // POST /api/v1/polls/:id/resolve - Resolve a poll with the winning outcome
        .route("/api/v1/polls/:id/resolve", post(polls::resolve_poll))

        // POST /api/v1/polls/:id/suspend - Temporarily suspend a poll
        .route("/api/v1/polls/:id/suspend", post(polls::suspend_poll))

        // POST /api/v1/polls/:id/cancel - Cancel a poll and refund bets
        .route("/api/v1/polls/:id/cancel", post(polls::cancel_poll))

        // POST /api/v1/polls/:poll_id/comments - Add a comment to a poll
        .route("/api/v1/polls/:poll_id/comments", post(comments::create_comment))

        // POST /api/v1/comments/:id/like - Toggle like on a comment
        .route("/api/v1/comments/:id/like", post(comments::toggle_like))

        // GET /api/v1/positions - Get the current user's open positions
        .route("/api/v1/positions", get(bets::get_user_positions))

        // POST /api/v1/challenges - Create a new challenge
        .route("/api/v1/challenges", post(challenges::create_challenge))

        // POST /api/v1/challenges/:id/accept - Accept a challenge
        .route("/api/v1/challenges/:id/accept", post(challenges::accept_challenge))

        // POST /api/v1/challenges/:id/resolve - Resolve a challenge
        .route("/api/v1/challenges/:id/resolve", post(challenges::resolve_challenge))

        // POST /api/v1/challenges/:id/cancel - Cancel a challenge
        .route("/api/v1/challenges/:id/cancel", post(challenges::cancel_challenge))

        // POST /api/v1/wagers - Create a new peer-to-peer wager
        .route("/api/v1/wagers", post(wagers::create_wager))

        // GET /api/v1/wagers - List the current user's wagers
        .route("/api/v1/wagers", get(wagers::list_wagers))

        // POST /api/v1/wagers/:id/accept - Accept a wager challenge
        .route("/api/v1/wagers/:id/accept", post(wagers::accept_wager))

        // POST /api/v1/wagers/:id/cancel - Cancel a wager
        .route("/api/v1/wagers/:id/cancel", post(wagers::cancel_wager))

        // POST /api/v1/paymaster/relay - Relay a gasless blockchain transaction
        .route("/api/v1/paymaster/relay", post(paymaster::relay))

        // GET /api/v1/paymaster/rate-limit - Check paymaster rate limit status
        .route("/api/v1/paymaster/rate-limit", get(paymaster::rate_limit_check))

        // GET /api/v1/paymaster/transactions/:id - Get a specific paymaster transaction
        .route("/api/v1/paymaster/transactions/:id", get(paymaster::get_tx))

        // GET /api/v1/paymaster/gas-expenditure - Get user's total gas expenditure
        .route("/api/v1/paymaster/gas-expenditure", get(paymaster::user_gas_expenditure))

        // GET /api/v1/wallet/balance - Get the current user's wallet balance
        .route("/api/v1/wallet/balance", get(wallet::get_user_balance))

        // GET /api/v1/wallet/transactions - Get the current user's transaction history
        .route("/api/v1/wallet/transactions", get(wallet::get_user_transactions))

        // GET /api/v1/notifications - List the current user's notifications
        .route("/api/v1/notifications", get(notifications::list_notifications))

        // POST /api/v1/notifications/:id/read - Mark a notification as read
        .route("/api/v1/notifications/:id/read", post(notifications::mark_notification_read))

        // POST /api/v1/notifications/read-all - Mark all notifications as read
        .route("/api/v1/notifications/read-all", post(notifications::mark_all_notifications_read))

        // GET /api/v1/notifications/unread-count - Get count of unread notifications
        .route("/api/v1/notifications/unread-count", get(notifications::get_unread_count))

        // GET /api/v1/users/me - Get the current user's profile
        .route("/api/v1/users/me", get(users::get_current_user))

        // GET /api/v1/users/me/portfolio - Get the current user's prediction portfolio
        .route("/api/v1/users/me/portfolio", get(users::get_portfolio))

        // P2P Bets routes
        // POST /api/v1/p2p-bets - Create a new P2P bet
        .route("/api/v1/p2p-bets", post(p2p_bets::create_bet))

        // GET /api/v1/p2p-bets - List P2P bets with filters
        .route("/api/v1/p2p-bets", get(p2p_bets::list_bets))

        // GET /api/v1/p2p-bets/:id - Get P2P bet details
        .route("/api/v1/p2p-bets/:id", get(p2p_bets::get_bet))

        // POST /api/v1/p2p-bets/:id/join - Join a P2P bet
        .route("/api/v1/p2p-bets/:id/join", post(p2p_bets::join_bet))

        // POST /api/v1/p2p-bets/:id/cancel - Cancel a P2P bet
        .route("/api/v1/p2p-bets/:id/cancel", post(p2p_bets::cancel_bet))

        // POST /api/v1/p2p-bets/:id/report-outcome - Report outcome for a P2P bet
        .route("/api/v1/p2p-bets/:id/report-outcome", post(p2p_bets::report_outcome))

        // POST /api/v1/p2p-bets/:id/confirm-outcome - Confirm outcome for a P2P bet
        .route("/api/v1/p2p-bets/:id/confirm-outcome", post(p2p_bets::confirm_outcome))

        // GET /api/v1/p2p-bets/:id/outcome-status - Get outcome status for a P2P bet
        .route("/api/v1/p2p-bets/:id/outcome-status", get(p2p_bets::get_outcome_status))

        // GET /api/v1/p2p-bets/share/:encrypted_id - Resolve shareable URL
        .route("/api/v1/p2p-bets/share/:encrypted_id", get(p2p_bets::resolve_shareable_url))

        // GET /api/v1/p2p-bets/my-positions - Get user's P2P bet positions
        .route("/api/v1/p2p-bets/my-positions", get(p2p_bets::get_my_positions))

        // GET /api/v1/p2p-bets/my-bets - Get user's created P2P bets
        .route("/api/v1/p2p-bets/my-bets", get(p2p_bets::get_my_bets))

        // Apply JWT authentication middleware - validates Bearer token
        // This runs BEFORE rate limiting (innermost layer runs first)
        .layer(middleware::from_fn_with_state(state.clone(), mw::auth::require_auth))

        // Apply general rate limiting after auth
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::rate_limit::rate_limit_general,
        ));

    // ---- M-PESA CALLBACK (commented out) ----
    // The M-Pesa callback doesn't need auth (Safaricom calls it directly)
    // but it's disabled since we're using Freighter wallet.
    // let mpesa_callback = Router::new()
    //     .route("/api/v1/wallet/mpesa/callback", post(wallet::mpesa_callback));

    // ---- ASSEMBLE THE FINAL ROUTER ----
    // Merge all route groups and apply global middleware.
    // .merge() combines routers - routes from all groups are available.
    // .layer() applies middleware to ALL routes in the router.
    Router::new()
        .merge(auth_routes)      // Authentication endpoints
        .merge(trading_routes)   // Trading/betting endpoints
        // .merge(mpesa_routes)  // M-Pesa routes (disabled)
        .merge(public)           // Public read-only endpoints
        .merge(websocket)        // WebSocket endpoint
        .merge(protected)        // JWT-protected endpoints
        // .merge(mpesa_callback) // M-Pesa callback (disabled)

        // Apply global middleware to ALL routes
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors)
                // Security headers
                .layer(SetResponseHeaderLayer::overriding(
                    header::STRICT_TRANSPORT_SECURITY,
                    HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::X_FRAME_OPTIONS,
                    HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::REFERRER_POLICY,
                    HeaderValue::from_static("strict-origin-when-cross-origin"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::HeaderName::from_static("permissions-policy"),
                    HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
                ))
                .layer(middleware::from_fn(mw::request_id::request_id)),
        )

        // Inject the shared application state into all handlers
        // Handlers access it via State<AppState> extractor
        .with_state(state)
} // end build_router
