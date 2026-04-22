use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{errors::AppError, state::AppState};

const SESSION_PREFIX: &str = "session:user:";
const SESSION_TTL: u64 = 7 * 24 * 3600; // 7 days

/// Cached user session data stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedUserSession {
    pub user_id: Uuid,
    pub email: String,
    pub username: String,
    pub is_active: bool,
    pub is_staff: bool,
}

/// Cache user session data in Redis with 7-day TTL
pub async fn cache_user_session(
    state: &AppState,
    user_id: Uuid,
    email: &str,
    username: &str,
    is_active: bool,
    is_staff: bool,
) -> Result<(), AppError> {
    let mut conn = state
        .redis()
        .get()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis connection: {e}")))?;

    let key = format!("{SESSION_PREFIX}{}", user_id);
    let session = CachedUserSession {
        user_id,
        email: email.to_string(),
        username: username.to_string(),
        is_active,
        is_staff,
    };
    let value = serde_json::to_string(&session)
        .map_err(|e| AppError::InternalServerError(format!("Serialize session: {e}")))?;

    let _: () = conn
        .set_ex(&key, value, SESSION_TTL)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis set: {e}")))?;

    Ok(())
}

/// Get user session from Redis cache, falling back to database if not found
pub async fn get_user_session(
    state: &AppState,
    user_id: Uuid,
) -> Result<Option<CachedUserSession>, AppError> {
    // Try cache first
    if let Some(cached) = get_cached_user_session(state, user_id).await? {
        return Ok(Some(cached));
    }

    // Fallback to database
    let row = sqlx::query(
        "SELECT id, email, username, is_active, is_staff FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(state.db())
    .await
    .map_err(|e| AppError::Database(e))?;

    if let Some(row) = row {
        let user_id: Uuid = row.get("id");
        let email: String = row.get("email");
        let username: String = row.get("username");
        let is_active: bool = row.get("is_active");
        let is_staff: bool = row.get("is_staff");

        let session = CachedUserSession {
            user_id,
            email: email.clone(),
            username: username.clone(),
            is_active,
            is_staff,
        };

        // Cache the session (ignore errors)
        let _ = cache_user_session(state, user_id, &email, &username, is_active, is_staff).await;

        Ok(Some(session))
    } else {
        Ok(None)
    }
}

/// Get user session from Redis cache only
async fn get_cached_user_session(
    state: &AppState,
    user_id: Uuid,
) -> Result<Option<CachedUserSession>, AppError> {
    let mut conn = state
        .redis()
        .get()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis connection: {e}")))?;

    let key = format!("{SESSION_PREFIX}{}", user_id);
    let raw: Option<String> = conn
        .get(&key)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis get: {e}")))?;

    match raw {
        None => Ok(None),
        Some(s) => {
            let cached = serde_json::from_str(&s)
                .map_err(|e| AppError::InternalServerError(format!("Deserialize session: {e}")))?;
            Ok(Some(cached))
        }
    }
}

/// Invalidate user session in cache (e.g., on logout)
pub async fn invalidate_user_session(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
    let mut conn = state
        .redis()
        .get()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis connection: {e}")))?;

    let key = format!("{SESSION_PREFIX}{}", user_id);
    let _: () = conn
        .del(&key)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Redis del: {e}")))?;

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
    async fn test_cache_and_get_user_session() {
        let state = setup_test_state().await;
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let username = "testuser";

        // Cache a session
        let result = cache_user_session(&state, user_id, email, username, true, false).await;
        assert!(result.is_ok(), "Failed to cache session");

        // Retrieve the session from cache
        let session = get_user_session(&state, user_id).await;
        assert!(session.is_ok(), "Failed to get session");

        let session = session.unwrap();
        assert!(session.is_some(), "Session not found in cache");

        let session = session.unwrap();
        assert_eq!(session.user_id, user_id);
        assert_eq!(session.email, email);
        assert_eq!(session.username, username);
        assert_eq!(session.is_active, true);
        assert_eq!(session.is_staff, false);
    }

    #[tokio::test]
    async fn test_invalidate_user_session() {
        let state = setup_test_state().await;
        let user_id = Uuid::new_v4();
        let email = "test2@example.com";
        let username = "testuser2";

        // Cache a session
        cache_user_session(&state, user_id, email, username, true, false)
            .await
            .expect("Failed to cache session");

        // Verify it exists
        let session = get_cached_user_session(&state, user_id).await;
        assert!(session.is_ok());
        assert!(session.unwrap().is_some());

        // Invalidate the session
        let result = invalidate_user_session(&state, user_id).await;
        assert!(result.is_ok(), "Failed to invalidate session");

        // Verify it's gone from cache
        let session = get_cached_user_session(&state, user_id).await;
        assert!(session.is_ok());
        assert!(session.unwrap().is_none(), "Session still in cache after invalidation");
    }

    #[tokio::test]
    async fn test_session_ttl() {
        let state = setup_test_state().await;
        let user_id = Uuid::new_v4();
        let email = "test3@example.com";
        let username = "testuser3";

        // Cache a session
        cache_user_session(&state, user_id, email, username, true, false)
            .await
            .expect("Failed to cache session");

        // Verify TTL is set (should be 7 days = 604800 seconds)
        let mut conn = state.redis().get().await.expect("Failed to get Redis connection");
        let key = format!("{SESSION_PREFIX}{}", user_id);
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .expect("Failed to get TTL");

        // TTL should be close to 7 days (allow some margin for execution time)
        assert!(ttl > 604700 && ttl <= 604800, "TTL not set correctly: {}", ttl);
    }
}
