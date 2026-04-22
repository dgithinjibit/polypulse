use anyhow::{Context, Result};
use redis::AsyncCommands;
use tracing::debug;

use crate::state::AppState;

/// Cache TTL for poll details (30 seconds)
const POLL_CACHE_TTL: u64 = 30;

/// Cache TTL for market prices (5 seconds)
const MARKET_PRICE_CACHE_TTL: u64 = 5;

/// Get cached poll details from Redis
pub async fn get_cached_poll(state: &AppState, poll_id: i64) -> Result<Option<String>> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("poll:{}", poll_id);
    let cached: Option<String> = conn.get(&key).await.unwrap_or(None);

    if cached.is_some() {
        debug!("Poll cache hit for poll_id={}", poll_id);
    }

    Ok(cached)
}

/// Cache poll details in Redis with 30-second TTL
pub async fn cache_poll(state: &AppState, poll_id: i64, data: &str) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("poll:{}", poll_id);
    let _: () = conn
        .set_ex(&key, data, POLL_CACHE_TTL)
        .await
        .context("Failed to cache poll")?;

    debug!("Cached poll_id={} with TTL={}s", poll_id, POLL_CACHE_TTL);
    Ok(())
}

/// Invalidate cached poll details
pub async fn invalidate_poll_cache(state: &AppState, poll_id: i64) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("poll:{}", poll_id);
    let _: () = conn.del(&key).await.context("Failed to invalidate poll cache")?;

    debug!("Invalidated cache for poll_id={}", poll_id);
    Ok(())
}

/// Get cached market prices from Redis
pub async fn get_cached_market_prices(state: &AppState, poll_id: i64) -> Result<Option<String>> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("market:prices:{}", poll_id);
    let cached: Option<String> = conn.get(&key).await.unwrap_or(None);

    if cached.is_some() {
        debug!("Market prices cache hit for poll_id={}", poll_id);
    }

    Ok(cached)
}

/// Cache market prices in Redis with 5-second TTL
pub async fn cache_market_prices(state: &AppState, poll_id: i64, data: &str) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("market:prices:{}", poll_id);
    let _: () = conn
        .set_ex(&key, data, MARKET_PRICE_CACHE_TTL)
        .await
        .context("Failed to cache market prices")?;

    debug!(
        "Cached market prices for poll_id={} with TTL={}s",
        poll_id, MARKET_PRICE_CACHE_TTL
    );
    Ok(())
}

/// Invalidate cached market prices
pub async fn invalidate_market_prices_cache(state: &AppState, poll_id: i64) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("market:prices:{}", poll_id);
    let _: () = conn
        .del(&key)
        .await
        .context("Failed to invalidate market prices cache")?;

    debug!("Invalidated market prices cache for poll_id={}", poll_id);
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, state::AppState};

    async fn setup_test_state() -> AppState {
        let config = Config::from_env().expect("Failed to load config");
        AppState::new(config).await.expect("Failed to create app state")
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_poll_cache_roundtrip() {
        let state = setup_test_state().await;
        let poll_id = 123;
        let data = r#"{"id":123,"title":"Test Poll"}"#;

        // Cache the poll
        cache_poll(&state, poll_id, data).await.expect("Failed to cache poll");

        // Retrieve from cache
        let cached = get_cached_poll(&state, poll_id).await.expect("Failed to get cached poll");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), data);

        // Invalidate cache
        invalidate_poll_cache(&state, poll_id).await.expect("Failed to invalidate cache");

        // Verify it's gone
        let cached = get_cached_poll(&state, poll_id).await.expect("Failed to get cached poll");
        assert!(cached.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_market_prices_cache_roundtrip() {
        let state = setup_test_state().await;
        let poll_id = 456;
        let data = r#"{"1":0.5,"2":0.5}"#;

        // Cache market prices
        cache_market_prices(&state, poll_id, data).await.expect("Failed to cache prices");

        // Retrieve from cache
        let cached = get_cached_market_prices(&state, poll_id).await.expect("Failed to get cached prices");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), data);

        // Invalidate cache
        invalidate_market_prices_cache(&state, poll_id).await.expect("Failed to invalidate cache");

        // Verify it's gone
        let cached = get_cached_market_prices(&state, poll_id).await.expect("Failed to get cached prices");
        assert!(cached.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_cache_ttl_expiry() {
        let state = setup_test_state().await;
        let poll_id = 789;
        let data = r#"{"id":789}"#;

        // Cache with 5-second TTL (market prices)
        cache_market_prices(&state, poll_id, data).await.expect("Failed to cache");

        // Should exist immediately
        let cached = get_cached_market_prices(&state, poll_id).await.expect("Failed to get cached");
        assert!(cached.is_some());

        // Wait for TTL to expire (6 seconds to be safe)
        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

        // Should be gone
        let cached = get_cached_market_prices(&state, poll_id).await.expect("Failed to get cached");
        assert!(cached.is_none());
    }
}
