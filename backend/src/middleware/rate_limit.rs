use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use crate::{errors::AppError, middleware::auth::AuthUser, state::AppState};

/// Rate limit configuration for different endpoint types
#[derive(Debug, Clone, Copy)]
pub enum RateLimitTier {
    /// Anonymous users: 60 requests per minute
    Anonymous,
    /// Authenticated users: 300 requests per minute
    Authenticated,
    /// Auth endpoints: 10 requests per minute
    Auth,
    /// Trading endpoints (bets, markets): 30 requests per minute
    Trading,
    /// M-Pesa endpoints: 5 requests per minute
    MPesa,
    /// P2P bet write endpoints (create, join, report): 20 requests per minute
    P2PWrite,
}

impl RateLimitTier {
    /// Returns the maximum number of requests allowed per minute
    fn limit(&self) -> u32 {
        match self {
            RateLimitTier::Anonymous => 60,
            RateLimitTier::Authenticated => 300,
            RateLimitTier::Auth => 10,
            RateLimitTier::Trading => 30,
            RateLimitTier::MPesa => 5,
            RateLimitTier::P2PWrite => 20,
        }
    }

    /// Returns the window duration in seconds (always 60 for per-minute limits)
    fn window_seconds(&self) -> u64 {
        60
    }
}

/// Extract identifier for rate limiting (IP for anonymous, user_id for authenticated)
fn get_rate_limit_key(req: &Request, tier: RateLimitTier) -> String {
    // Try to get user ID from extensions (set by auth middleware)
    if let Some(auth_user) = req.extensions().get::<AuthUser>() {
        format!("rate_limit:{}:user:{}", tier_name(tier), auth_user.0.sub)
    } else {
        // Fall back to IP address for anonymous users
        let ip = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next())
            .unwrap_or("unknown");
        format!("rate_limit:{}:ip:{}", tier_name(tier), ip)
    }
}

fn tier_name(tier: RateLimitTier) -> &'static str {
    match tier {
        RateLimitTier::Anonymous => "anon",
        RateLimitTier::Authenticated => "auth",
        RateLimitTier::Auth => "auth_endpoint",
        RateLimitTier::Trading => "trading",
        RateLimitTier::MPesa => "mpesa",
        RateLimitTier::P2PWrite => "p2p_write",
    }
}

/// Check rate limit using Redis sliding window algorithm
async fn check_rate_limit(
    state: &AppState,
    key: &str,
    tier: RateLimitTier,
) -> Result<(), AppError> {
    let mut conn = state
        .redis()
        .get()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis connection error: {e}")))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let window_start = now - tier.window_seconds();

    // Use Redis sorted set for sliding window
    // Remove old entries outside the window
    let _: () = conn
        .zrembyscore(key, 0, window_start as i64)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis error: {e}")))?;

    // Count requests in current window
    let count: u32 = conn
        .zcard(key)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis error: {e}")))?;

    debug!(
        "Rate limit check: key={}, count={}, limit={}",
        key,
        count,
        tier.limit()
    );

    if count >= tier.limit() {
        warn!("Rate limit exceeded: key={}, count={}", key, count);
        return Err(AppError::TooManyRequests(format!(
            "Rate limit exceeded. Maximum {} requests per minute allowed.",
            tier.limit()
        )));
    }

    // Add current request to the window
    let _: () = conn
        .zadd(key, now as i64, now)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis error: {e}")))?;

    // Set expiry on the key to clean up old data
    let _: () = conn
        .expire(key, tier.window_seconds() as i64 + 10)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis error: {e}")))?;

    Ok(())
}

/// Middleware factory for rate limiting with specified tier
pub async fn rate_limit_with_tier(
    tier: RateLimitTier,
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = get_rate_limit_key(&req, tier);
    check_rate_limit(&state, &key, tier).await?;
    Ok(next.run(req).await)
}

/// Middleware for anonymous/general rate limiting (60 req/min for anon, 300 for auth)
pub async fn rate_limit_general(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Determine tier based on whether user is authenticated
    let tier = if req.extensions().get::<AuthUser>().is_some() {
        RateLimitTier::Authenticated
    } else {
        RateLimitTier::Anonymous
    };

    let key = get_rate_limit_key(&req, tier);
    check_rate_limit(&state, &key, tier).await?;
    Ok(next.run(req).await)
}

/// Middleware for auth endpoints (10 req/min)
pub async fn rate_limit_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = get_rate_limit_key(&req, RateLimitTier::Auth);
    check_rate_limit(&state, &key, RateLimitTier::Auth).await?;
    Ok(next.run(req).await)
}

/// Middleware for trading endpoints (30 req/min)
pub async fn rate_limit_trading(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = get_rate_limit_key(&req, RateLimitTier::Trading);
    check_rate_limit(&state, &key, RateLimitTier::Trading).await?;
    Ok(next.run(req).await)
}

/// Middleware for M-Pesa endpoints (5 req/min)
pub async fn rate_limit_mpesa(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = get_rate_limit_key(&req, RateLimitTier::MPesa);
    check_rate_limit(&state, &key, RateLimitTier::MPesa).await?;
    Ok(next.run(req).await)
}

/// Middleware for P2P bet write endpoints (20 req/min)
/// Applied to create, join, cancel, report-outcome, confirm-outcome endpoints
/// to prevent spam and abuse of the P2P betting system.
pub async fn rate_limit_p2p_write(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = get_rate_limit_key(&req, RateLimitTier::P2PWrite);
    check_rate_limit(&state, &key, RateLimitTier::P2PWrite).await?;
    Ok(next.run(req).await)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_tier_limits() {
        assert_eq!(RateLimitTier::Anonymous.limit(), 60);
        assert_eq!(RateLimitTier::Authenticated.limit(), 300);
        assert_eq!(RateLimitTier::Auth.limit(), 10);
        assert_eq!(RateLimitTier::Trading.limit(), 30);
        assert_eq!(RateLimitTier::MPesa.limit(), 5);
        assert_eq!(RateLimitTier::P2PWrite.limit(), 20);
    }

    #[test]
    fn test_rate_limit_tier_window() {
        assert_eq!(RateLimitTier::Anonymous.window_seconds(), 60);
        assert_eq!(RateLimitTier::Authenticated.window_seconds(), 60);
        assert_eq!(RateLimitTier::Auth.window_seconds(), 60);
        assert_eq!(RateLimitTier::Trading.window_seconds(), 60);
        assert_eq!(RateLimitTier::MPesa.window_seconds(), 60);
        assert_eq!(RateLimitTier::P2PWrite.window_seconds(), 60);
    }

    #[test]
    fn test_tier_name() {
        assert_eq!(tier_name(RateLimitTier::Anonymous), "anon");
        assert_eq!(tier_name(RateLimitTier::Authenticated), "auth");
        assert_eq!(tier_name(RateLimitTier::Auth), "auth_endpoint");
        assert_eq!(tier_name(RateLimitTier::Trading), "trading");
        assert_eq!(tier_name(RateLimitTier::MPesa), "mpesa");
        assert_eq!(tier_name(RateLimitTier::P2PWrite), "p2p_write");
    }
}
