#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    async fn setup_test_state() -> AppState {
        // This would need actual Redis and DB setup for integration tests
        // For now, this is a placeholder for the test structure
        todo!("Setup test state with Redis and DB")
    }

    #[tokio::test]
    async fn test_rate_limit_anonymous_user() {
        let state = setup_test_state().await;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_general,
            ))
            .with_state(state);

        // Make 60 requests (the limit for anonymous users)
        for i in 0..60 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/test")
                        .header("x-forwarded-for", "192.168.1.1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Request {} should succeed",
                i + 1
            );
        }

        // 61st request should be rate limited
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("x-forwarded-for", "192.168.1.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_rate_limit_auth_endpoint() {
        let state = setup_test_state().await;

        let app = Router::new()
            .route("/auth/login", get(test_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_auth,
            ))
            .with_state(state);

        // Make 10 requests (the limit for auth endpoints)
        for i in 0..10 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/auth/login")
                        .header("x-forwarded-for", "192.168.1.1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Request {} should succeed",
                i + 1
            );
        }

        // 11th request should be rate limited
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/login")
                    .header("x-forwarded-for", "192.168.1.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_rate_limit_trading_endpoint() {
        let state = setup_test_state().await;

        let app = Router::new()
            .route("/bets", get(test_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_trading,
            ))
            .with_state(state);

        // Make 30 requests (the limit for trading endpoints)
        for i in 0..30 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/bets")
                        .header("x-forwarded-for", "192.168.1.1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Request {} should succeed",
                i + 1
            );
        }

        // 31st request should be rate limited
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/bets")
                    .header("x-forwarded-for", "192.168.1.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let state = setup_test_state().await;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_general,
            ))
            .with_state(state);

        // Make 60 requests from IP 1
        for _ in 0..60 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/test")
                        .header("x-forwarded-for", "192.168.1.1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }

        // Request from IP 2 should still work
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("x-forwarded-for", "192.168.1.2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
