use axum::{
    extract::{Extension, State},
    Json,
};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{errors::AppResult, middleware::auth::AuthUser, state::AppState};

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub wallet_address: Option<String>,
    pub balance: i64,
    pub referral_code: String,
    pub reputation_score: i32,
}

/// GET /api/v1/users/me
pub async fn get_current_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<UserProfile>> {
    let user_id = auth_user.0.sub;

    let row = sqlx::query(
        r#"
        SELECT u.id, u.email, u.username, u.balance,
               COALESCE(p.referral_code, '') as referral_code,
               0 as reputation_score
        FROM users u
        LEFT JOIN profiles p ON p.user_id = u.id
        WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db())
    .await?;

    Ok(Json(UserProfile {
        id: row.get("id"),
        email: row.get("email"),
        username: row.get("username"),
        wallet_address: None,
        balance: row.get::<f64, _>("balance") as i64,
        referral_code: row.get("referral_code"),
        reputation_score: row.get("reputation_score"),
    }))
}

#[derive(Debug, Serialize)]
pub struct PortfolioStats {
    pub total_at_risk: i64,
    pub total_yield_earned: i64,
    pub win_rate: f64,
    pub active_wagers: i64,
    pub resolved_wagers: i64,
}

/// GET /api/v1/users/me/portfolio
pub async fn get_portfolio(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<PortfolioStats>> {
    let user_id = auth_user.0.sub;

    let active_row = sqlx::query(
        r#"
        SELECT COUNT(*) AS cnt FROM wagers w
        LEFT JOIN wager_participants wp ON wp.wager_id = w.id AND wp.user_id = $1
        WHERE (w.creator_id = $1 OR wp.user_id = $1) AND w.status = 'active'
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db())
    .await?;
    let active: i64 = active_row.get("cnt");

    let resolved_row = sqlx::query(
        r#"
        SELECT COUNT(*) AS cnt FROM wagers w
        LEFT JOIN wager_participants wp ON wp.wager_id = w.id AND wp.user_id = $1
        WHERE (w.creator_id = $1 OR wp.user_id = $1) AND w.status = 'resolved'
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db())
    .await?;
    let resolved: i64 = resolved_row.get("cnt");

    let risk_row = sqlx::query(
        r#"
        SELECT COALESCE(SUM(wp.amount), 0) AS total FROM wager_participants wp
        JOIN wagers w ON w.id = wp.wager_id
        WHERE wp.user_id = $1 AND w.status = 'active'
        "#,
    )
    .bind(user_id)
    .fetch_one(state.db())
    .await?;
    let total_at_risk: i64 = risk_row.get("total");

    Ok(Json(PortfolioStats {
        total_at_risk,
        total_yield_earned: 0, // populated in task 8.3
        win_rate: 0.0,         // populated in task 11.2
        active_wagers: active,
        resolved_wagers: resolved,
    }))
}
