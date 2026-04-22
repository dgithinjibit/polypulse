#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, state::AppState};

    async fn create_test_state() -> AppState {
        let config = Config {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
            jwt_secret: "test_secret".to_string(),
            cors_origins: vec!["http://localhost:3000".to_string()],
            rust_log: "info".to_string(),
            port: 8080,
            frontend_url: "http://localhost:3000".to_string(),
            web3auth_client_id: None,
            mpesa_env: None,
            mpesa_consumer_key: None,
            mpesa_consumer_secret: None,
            mpesa_shortcode: None,
            mpesa_passkey: None,
            mpesa_callback_url: None,
            email_host: None,
            email_port: 587,
            email_user: None,
            email_password: None,
            email_from: "test@example.com".to_string(),
        };

        // Create a minimal redis pool for testing
        let redis_cfg = deadpool_redis::Config::from_url(&config.redis_url);
        let redis = redis_cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .unwrap();

        // Create minimal DB pool (won't be used in cache tests)
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&config.database_url)
            .await
            .unwrap();

        let connection_registry = crate::ws::ConnectionRegistry::new();
        let broadcast_hub = crate::ws::BroadcastHub::new(connection_registry.clone());
        let market_hub = crate::ws::MarketHub::new(broadcast_hub.clone());

        AppState {
            inner: std::sync::Arc::new(crate::state::Inner {
                db,
                redis,
                config,
            }),
            market_hub,
            connection_registry,
            broadcast_hub,
        }
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_poll_cache_roundtrip() {
        let state = create_test_state().await;
        let poll_id = 123;
        let test_data = r#"{"id":123,"title":"Test Poll"}"#;

        // Cache the data
        let cache_result = cache_poll(&state, poll_id, test_data).await;
        assert!(cache_result.is_ok(), "Should cache poll successfully");

        // Retrieve from cache
        let cached = get_cached_poll(&state, poll_id).await;
        assert!(cached.is_ok(), "Should retrieve from cache");
        assert_eq!(
            cached.unwrap(),
            Some(test_data.to_string()),
            "Cached data should match"
        );

        // Invalidate cache
        let invalidate_result = invalidate_poll_cache(&state, poll_id).await;
        assert!(invalidate_result.is_ok(), "Should invalidate cache");

        // Verify cache is empty
        let cached_after = get_cached_poll(&state, poll_id).await;
        assert!(cached_after.is_ok(), "Should query cache");
        assert_eq!(
            cached_after.unwrap(),
            None,
            "Cache should be empty after invalidation"
        );
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_market_prices_cache_roundtrip() {
        let state = create_test_state().await;
        let poll_id = 456;
        let test_data = r#"{"prices":{"1":0.5,"2":0.5}}"#;

        // Cache the data
        let cache_result = cache_market_prices(&state, poll_id, test_data).await;
        assert!(
            cache_result.is_ok(),
            "Should cache market prices successfully"
        );

        // Retrieve from cache
        let cached = get_cached_market_prices(&state, poll_id).await;
        assert!(cached.is_ok(), "Should retrieve from cache");
        assert_eq!(
            cached.unwrap(),
            Some(test_data.to_string()),
            "Cached data should match"
        );

        // Invalidate cache
        let invalidate_result = invalidate_market_prices_cache(&state, poll_id).await;
        assert!(invalidate_result.is_ok(), "Should invalidate cache");

        // Verify cache is empty
        let cached_after = get_cached_market_prices(&state, poll_id).await;
        assert!(cached_after.is_ok(), "Should query cache");
        assert_eq!(
            cached_after.unwrap(),
            None,
            "Cache should be empty after invalidation"
        );
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_cache_ttl_expiry() {
        let state = create_test_state().await;
        let poll_id = 789;
        let test_data = r#"{"id":789,"title":"TTL Test"}"#;

        // Cache the data
        cache_poll(&state, poll_id, test_data).await.unwrap();

        // Verify it's cached
        let cached = get_cached_poll(&state, poll_id).await.unwrap();
        assert_eq!(cached, Some(test_data.to_string()));

        // Wait for TTL to expire (30 seconds for polls)
        tokio::time::sleep(tokio::time::Duration::from_secs(31)).await;

        // Verify cache is empty after TTL
        let cached_after = get_cached_poll(&state, poll_id).await.unwrap();
        assert_eq!(
            cached_after, None,
            "Cache should be empty after TTL expiry"
        );
    }
}
