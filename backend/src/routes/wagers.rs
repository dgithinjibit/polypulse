use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    state::AppState,
};

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateWagerRequest {
    pub description: String,
    pub resolution_criteria: String,
    pub amount: i64,
    pub max_participants: i32,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub is_public: bool,
    pub trusted_judge_id: Option<Uuid>,
}

impl CreateWagerRequest {
    fn validate(&self) -> Result<(), AppError> {
        if self.description.is_empty() || self.description.len() > 500 {
            return Err(AppError::BadRequest("Description must be 1-500 characters".into()));
        }
        if self.resolution_criteria.len() < 10 || self.resolution_criteria.len() > 2000 {
            return Err(AppError::BadRequest(
                "Resolution criteria must be 10-2000 characters".into(),
            ));
        }
        if self.amount < 1 || self.amount > 1_000_000 {
            return Err(AppError::BadRequest("Amount must be 1-1000000".into()));
        }
        if self.max_participants < 2 || self.max_participants > 10 {
            return Err(AppError::BadRequest("max_participants must be 2-10".into()));
        }
        if self.expires_at <= Utc::now() {
            return Err(AppError::BadRequest("expires_at must be in the future".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct WagerResponse {
    pub id: Uuid,
    pub wager_link: String,
    pub description: String,
    pub resolution_criteria: String,
    pub amount: i64,
    pub max_participants: i32,
    pub status: String,
    pub is_public: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/wagers — create a new private wager
pub async fn create_wager(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<CreateWagerRequest>,
) -> AppResult<(StatusCode, Json<WagerResponse>)> {
    body.validate()?;

    let wager_id = Uuid::new_v4();
    let creator_id = auth_user.0.sub;

    let row = sqlx::query(
        r#"
        INSERT INTO wagers (
            id, creator_id, description, resolution_criteria,
            amount, max_participants, status, is_public,
            trusted_judge_id, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8, $9)
        RETURNING id, description, resolution_criteria, amount,
                  max_participants, status, is_public, expires_at, created_at
        "#,
    )
    .bind(wager_id)
    .bind(creator_id)
    .bind(&body.description)
    .bind(&body.resolution_criteria)
    .bind(body.amount)
    .bind(body.max_participants)
    .bind(body.is_public)
    .bind(body.trusted_judge_id)
    .bind(body.expires_at)
    .fetch_one(state.db())
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(row_to_wager_response(&row)),
    ))
}

/// GET /api/v1/wagers/:id — fetch wager details via link ID (public)
pub async fn get_wager_by_link(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WagerResponse>> {
    let row = sqlx::query(
        r#"
        SELECT id, description, resolution_criteria, amount,
               max_participants, status, is_public, expires_at, created_at
        FROM wagers WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Wager {id} not found")))?;

    Ok(Json(row_to_wager_response(&row)))
}

/// GET /api/v1/wagers — list wagers for the authenticated user
pub async fn list_wagers(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<Vec<WagerResponse>>> {
    let user_id = auth_user.0.sub;

    let rows = sqlx::query(
        r#"
        SELECT DISTINCT w.id, w.description, w.resolution_criteria, w.amount,
               w.max_participants, w.status, w.is_public, w.expires_at, w.created_at
        FROM wagers w
        LEFT JOIN wager_participants wp ON wp.wager_id = w.id AND wp.user_id = $1
        WHERE w.creator_id = $1 OR wp.user_id = $1
        ORDER BY w.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(state.db())
    .await?;

    Ok(Json(rows.iter().map(row_to_wager_response).collect()))
}

/// POST /api/v1/wagers/:id/accept — accept a wager
pub async fn accept_wager(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = auth_user.0.sub;

    // Fetch wager
    let wager_row = sqlx::query(
        "SELECT id, creator_id, amount, status, max_participants FROM wagers WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Wager {id} not found")))?;

    let status: String = wager_row.get("status");
    let creator_id: Uuid = wager_row.get("creator_id");
    let amount: i64 = wager_row.get("amount");
    let max_participants: i32 = wager_row.get("max_participants");

    if status != "pending" {
        return Err(AppError::Conflict("Wager is not open for acceptance".to_string()));
    }
    if creator_id == user_id {
        return Err(AppError::BadRequest("Creator cannot accept their own wager".to_string()));
    }

    // Deduct balance and add participant atomically — balance check inside transaction
    // to prevent TOCTOU race condition
    let mut tx = state.db().begin().await?;

    // Re-check and deduct balance atomically with FOR UPDATE
    let balance_row = sqlx::query(
        "SELECT balance FROM users WHERE id = $1 FOR UPDATE"
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    let balance: i64 = balance_row.get("balance");

    if balance < amount {
        tx.rollback().await?;
        return Err(AppError::BadRequest("Insufficient balance".to_string()));
    }

    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2 AND balance >= $1")
        .bind(amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO wager_participants (id, wager_id, user_id, amount) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(id)
    .bind(user_id)
    .bind(amount)
    .execute(&mut *tx)
    .await?;

    // Count participants inside transaction for accurate capacity check
    let count_row =
        sqlx::query("SELECT COUNT(*) AS cnt FROM wager_participants WHERE wager_id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    let participant_count: i64 = count_row.get("cnt");

    if participant_count > max_participants as i64 {
        tx.rollback().await?;
        return Err(AppError::Conflict("Wager is full".to_string()));
    }

    // Lock wager if max participants reached
    if participant_count >= max_participants as i64 {
        sqlx::query("UPDATE wagers SET status = 'active', updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "message": "Wager accepted successfully",
        "wager_id": id,
    })))
}

/// POST /api/v1/wagers/:id/cancel — cancel an unaccepted wager
pub async fn cancel_wager(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = auth_user.0.sub;

    let wager_row =
        sqlx::query("SELECT id, creator_id, status FROM wagers WHERE id = $1")
            .bind(id)
            .fetch_optional(state.db())
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Wager {id} not found")))?;

    let creator_id: Uuid = wager_row.get("creator_id");
    let status: String = wager_row.get("status");

    if creator_id != user_id {
        return Err(AppError::Forbidden("Only the creator can cancel this wager".to_string()));
    }
    if status == "active" {
        return Err(AppError::Conflict("Cannot cancel an accepted wager".to_string()));
    }

    sqlx::query("UPDATE wagers SET status = 'cancelled', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(state.db())
        .await?;

    Ok(Json(serde_json::json!({
        "message": "Wager cancelled",
        "wager_id": id,
    })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_wager_link(id: Uuid) -> String {
    let base = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:5173".to_string());
    format!("{base}/wager/{id}")
}

fn row_to_wager_response(row: &sqlx::postgres::PgRow) -> WagerResponse {
    let id: Uuid = row.get("id");
    WagerResponse {
        id,
        wager_link: build_wager_link(id),
        description: row.get("description"),
        resolution_criteria: row.get("resolution_criteria"),
        amount: row.get("amount"),
        max_participants: row.get("max_participants"),
        status: row.get("status"),
        is_public: row.get("is_public"),
        expires_at: row.get("expires_at"),
        created_at: row.get("created_at"),
    }
}
