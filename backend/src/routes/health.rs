use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::state::AppState;

/// GET /health — liveness + readiness probe.
/// Checks DB and Redis connectivity.
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let db_ok = sqlx::query("SELECT 1 AS one")
        .execute(state.db())
        .await
        .is_ok();

    let redis_ok = {
        match state.redis().get().await {
            Ok(mut conn) => {
                let pong: Result<String, _> = redis::cmd("PING").query_async(&mut conn).await;
                pong.is_ok()
            }
            Err(_) => false,
        }
    };

    let status = if db_ok && redis_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(json!({
            "status": if db_ok && redis_ok { "ok" } else { "degraded" },
            "database": if db_ok { "ok" } else { "error" },
            "redis": if redis_ok { "ok" } else { "error" },
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}
