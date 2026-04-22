//! Notifications API routes.

use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct NotificationListQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct NotificationWithActor {
    pub id: i64,
    pub user_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub notification_type: String,
    pub message: String,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub notifications: Vec<NotificationWithActor>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct StatusMessage {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub count: i64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/notifications
///
/// List notifications for the authenticated user with pagination.
/// Notifications are ordered by created_at descending (newest first).
/// Includes actor details (username) when available.
pub async fn list_notifications(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Query(query): Query<NotificationListQuery>,
) -> AppResult<Json<NotificationListResponse>> {
    let user_id = claims.sub;

    // Query notifications with actor details
    let rows = sqlx::query(
        r#"
        SELECT 
            n.id, n.user_id, n.actor_id, n.notification_type, n.message, n.is_read, n.created_at,
            u.username as actor_username
        FROM notifications n
        LEFT JOIN users u ON n.actor_id = u.id
        WHERE n.user_id = $1
        ORDER BY n.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(query.limit)
    .bind(query.offset)
    .fetch_all(state.db())
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to fetch notifications: {e}")))?;

    // Count total notifications for user
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM notifications WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db())
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to count notifications: {e}")))?;

    // Transform to response format
    let notifications_with_actor: Vec<NotificationWithActor> = rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            NotificationWithActor {
                id: row.get("id"),
                user_id: row.get("user_id"),
                actor_id: row.get("actor_id"),
                actor_username: row.get("actor_username"),
                notification_type: row.get("notification_type"),
                message: row.get("message"),
                is_read: row.get("is_read"),
                created_at: row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(NotificationListResponse {
        notifications: notifications_with_actor,
        total,
        limit: query.limit,
        offset: query.offset,
    }))
}

/// POST /api/v1/notifications/:id/read
///
/// Mark a specific notification as read.
/// Verifies the notification belongs to the authenticated user.
pub async fn mark_notification_read(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<StatusMessage>> {
    let user_id = claims.sub;

    // Update notification if it belongs to the user
    let result = sqlx::query(
        r#"
        UPDATE notifications
        SET is_read = true
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(state.db())
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to update notification: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Notification not found or does not belong to user".to_string(),
        ));
    }

    Ok(Json(StatusMessage {
        message: "Notification marked as read".to_string(),
    }))
}

/// POST /api/v1/notifications/read-all
///
/// Mark all unread notifications as read for the authenticated user.
pub async fn mark_all_notifications_read(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
) -> AppResult<Json<StatusMessage>> {
    let user_id = claims.sub;

    // Update all unread notifications for the user
    let result = sqlx::query(
        r#"
        UPDATE notifications
        SET is_read = true
        WHERE user_id = $1 AND is_read = false
        "#,
    )
    .bind(user_id)
    .execute(state.db())
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to update notifications: {e}")))?;

    let count = result.rows_affected();

    Ok(Json(StatusMessage {
        message: format!("Marked {} notification(s) as read", count),
    }))
}

/// GET /api/v1/notifications/unread-count
///
/// Get the count of unread notifications for the authenticated user.
pub async fn get_unread_count(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
) -> AppResult<Json<UnreadCountResponse>> {
    let user_id = claims.sub;

    // Count unread notifications
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM notifications
        WHERE user_id = $1 AND is_read = false
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db())
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to count unread notifications: {e}")))?;

    Ok(Json(UnreadCountResponse { count }))
}
