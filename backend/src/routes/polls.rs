use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    lmsr,
    middleware::auth::AuthUser,
    models::{Bet, Market, MarketPosition, Poll, PollOption, Profile, User},
    state::AppState,
};

// ─── Request/Response Types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreatePollRequest {
    pub title: String,
    pub description: String,
    pub options: Vec<String>,
    pub closes_at: DateTime<Utc>,
    pub category_id: Option<i64>,
    pub is_free: Option<bool>,
    pub resolution_criteria: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PollListParams {
    pub category_id: Option<i64>,
    pub status: Option<String>,
    pub creator_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreatePollResponse {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub closes_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub options: Vec<PollOptionResponse>,
    pub market: MarketResponse,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PollOptionResponse {
    pub id: i64,
    pub text: String,
    pub order: i16,
    pub price: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketResponse {
    pub id: i64,
    pub liquidity_b: f64,
}

#[derive(Debug, Serialize)]
pub struct PollListItem {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub closes_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub creator_id: Uuid,
    pub creator_username: String,
    pub category_id: Option<i64>,
    pub is_free: bool,
    pub options: Vec<PollOptionResponse>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PollDetail {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub closes_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub creator_id: Uuid,
    pub category_id: Option<i64>,
    pub is_free: bool,
    pub resolution_criteria: String,
    pub winning_option_id: Option<i64>,
    pub options: Vec<PollOptionResponse>,
    pub market: MarketResponse,
    pub stats: PollStats,
    pub user_position: Option<UserPosition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PollStats {
    pub total_bets: i64,
    pub unique_bettors: i64,
    pub total_volume: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPosition {
    pub shares: std::collections::HashMap<i64, f64>,
    pub spent: std::collections::HashMap<i64, f64>,
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

/// POST /api/v1/polls - Create a new poll/market
pub async fn create_poll(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Json(req): Json<CreatePollRequest>,
) -> AppResult<Json<CreatePollResponse>> {
    // Validate request
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title cannot be empty".to_string()));
    }
    if req.options.len() < 2 {
        return Err(AppError::BadRequest("At least 2 options required".to_string()));
    }
    if req.options.len() > 10 {
        return Err(AppError::BadRequest("Maximum 10 options allowed".to_string()));
    }
    if req.closes_at <= Utc::now() {
        return Err(AppError::BadRequest("closes_at must be in the future".to_string()));
    }

    let db = state.db();
    let user_id = claims.sub;

    // Check daily poll creation limit
    let mut tx = db.begin().await?;
    
    let profile = sqlx::query_as::<_, Profile>(
        "SELECT * FROM profiles WHERE user_id = $1 FOR UPDATE"
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("User profile not found".to_string()))?;

    // Check if user can create poll today
    let today = Utc::now().date_naive();
    let can_create = if let Some(last_date) = profile.last_poll_created_date {
        if last_date == today {
            profile.polls_created_today < 1
        } else {
            true // New day, reset counter
        }
    } else {
        true // First poll ever
    };

    if !can_create {
        return Err(AppError::Forbidden("Daily poll creation limit reached (1 per day)".to_string()));
    }

    // Insert poll
    let poll = sqlx::query_as::<_, Poll>(
        r#"
        INSERT INTO polls (creator_id, title, description, category_id, status, is_free, closes_at, resolution_criteria)
        VALUES ($1, $2, $3, $4, 'open', $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.category_id)
    .bind(req.is_free.unwrap_or(true))
    .bind(req.closes_at)
    .bind(req.resolution_criteria.unwrap_or_default())
    .fetch_one(&mut *tx)
    .await?;

    // Insert poll options
    let mut options = Vec::new();
    for (idx, text) in req.options.iter().enumerate() {
        let option = sqlx::query_as::<_, PollOption>(
            r#"
            INSERT INTO poll_options (poll_id, text, "order", is_yes)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(poll.id)
        .bind(text)
        .bind(idx as i16)
        .bind(idx == 0) // First option is YES in binary markets
        .fetch_one(&mut *tx)
        .await?;
        options.push(option);
    }

    // Create market with initial liquidity
    let liquidity_b = 100.0;
    let shares_outstanding = serde_json::json!({});
    
    let market = sqlx::query_as::<_, Market>(
        r#"
        INSERT INTO markets (poll_id, liquidity_b, shares_outstanding)
        VALUES ($1, $2, $3)
        RETURNING *
        "#
    )
    .bind(poll.id)
    .bind(liquidity_b)
    .bind(&shares_outstanding)
    .fetch_one(&mut *tx)
    .await?;

    // Update profile poll creation counter
    let new_count = if profile.last_poll_created_date == Some(today) {
        profile.polls_created_today + 1
    } else {
        1
    };

    sqlx::query(
        "UPDATE profiles SET polls_created_today = $1, last_poll_created_date = $2 WHERE user_id = $3"
    )
    .bind(new_count)
    .bind(today)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Calculate initial prices (all equal for new market)
    let initial_shares = vec![0.0; options.len()];
    let option_responses: Vec<PollOptionResponse> = options
        .iter()
        .enumerate()
        .map(|(idx, opt)| PollOptionResponse {
            id: opt.id,
            text: opt.text.clone(),
            order: opt.order,
            price: lmsr::calculate_price(&initial_shares, idx, liquidity_b),
        })
        .collect();

    Ok(Json(CreatePollResponse {
        id: poll.id,
        title: poll.title,
        description: poll.description,
        status: poll.status,
        closes_at: poll.closes_at,
        created_at: poll.created_at,
        options: option_responses,
        market: MarketResponse {
            id: market.id,
            liquidity_b: market.liquidity_b,
        },
    }))
}


/// GET /api/v1/polls - List polls with filtering and pagination
pub async fn list_polls(
    State(state): State<AppState>,
    Query(params): Query<PollListParams>,
) -> AppResult<Json<Vec<PollListItem>>> {
    let db = state.db();
    
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let mut query = String::from(
        r#"
        SELECT p.*, m.id as market_id, m.liquidity_b, m.shares_outstanding, u.username as creator_username
        FROM polls p
        INNER JOIN markets m ON m.poll_id = p.id
        INNER JOIN users u ON u.id = p.creator_id
        WHERE 1=1
        "#
    );

    let mut bind_count = 0;

    if params.category_id.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND p.category_id = ${}", bind_count));
    }

    if params.status.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND p.status::text = ${}", bind_count));
    }

    if params.creator_id.is_some() {
        bind_count += 1;
        query.push_str(&format!(" AND p.creator_id = ${}", bind_count));
    }

    query.push_str(" ORDER BY p.created_at DESC");
    
    bind_count += 1;
    query.push_str(&format!(" LIMIT ${}", bind_count));
    
    bind_count += 1;
    query.push_str(&format!(" OFFSET ${}", bind_count));

    // Execute query with dynamic bindings
    let mut sql_query = sqlx::query(&query);

    if let Some(category_id) = params.category_id {
        sql_query = sql_query.bind(category_id);
    }
    if let Some(ref status) = params.status {
        sql_query = sql_query.bind(status);
    }
    if let Some(creator_id) = params.creator_id {
        sql_query = sql_query.bind(creator_id);
    }
    sql_query = sql_query.bind(limit).bind(offset);

    let rows = sql_query.fetch_all(db).await?;

    let mut result = Vec::new();

    for row in rows {
        let poll_id: i64 = row.try_get("id")?;
        let title: String = row.try_get("title")?;
        let description: String = row.try_get("description")?;
        let status: String = row.try_get("status")?;
        let closes_at: DateTime<Utc> = row.try_get("closes_at")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let creator_id: Uuid = row.try_get("creator_id")?;
        let creator_username: String = row.try_get("creator_username")?;
        let category_id: Option<i64> = row.try_get("category_id")?;
        let is_free: bool = row.try_get("is_free")?;
        let liquidity_b: f64 = row.try_get("liquidity_b")?;
        let shares_outstanding: serde_json::Value = row.try_get("shares_outstanding")?;

        // Fetch options for this poll
        let options = sqlx::query_as::<_, PollOption>(
            r#"SELECT * FROM poll_options WHERE poll_id = $1 ORDER BY "order""#
        )
        .bind(poll_id)
        .fetch_all(db)
        .await?;

        // Parse shares from JSON
        let shares_map: std::collections::HashMap<String, f64> = 
            serde_json::from_value(shares_outstanding).unwrap_or_default();
        
        let shares: Vec<f64> = options
            .iter()
            .map(|opt| shares_map.get(&opt.id.to_string()).copied().unwrap_or(0.0))
            .collect();

        // Calculate current prices
        let option_responses: Vec<PollOptionResponse> = options
            .iter()
            .enumerate()
            .map(|(idx, opt)| PollOptionResponse {
                id: opt.id,
                text: opt.text.clone(),
                order: opt.order,
                price: lmsr::calculate_price(&shares, idx, liquidity_b),
            })
            .collect();

        result.push(PollListItem {
            id: poll_id,
            title,
            description,
            status,
            closes_at,
            created_at,
            creator_id,
            creator_username,
            category_id,
            is_free,
            options: option_responses,
        });
    }

    Ok(Json(result))
}


/// GET /api/v1/polls/:id - Get detailed poll information
pub async fn get_poll_detail(
    State(state): State<AppState>,
    Path(poll_id): Path<i64>,
    auth_user: Option<Extension<AuthUser>>,
) -> AppResult<Json<PollDetail>> {
    let db = state.db();

    // Try to get from cache first (only for non-authenticated requests to avoid user-specific data issues)
    if auth_user.is_none() {
        if let Ok(Some(cached)) = crate::services::cache::get_cached_poll(&state, poll_id).await {
            if let Ok(poll_detail) = serde_json::from_str::<PollDetail>(&cached) {
                return Ok(Json(poll_detail));
            }
        }
    }

    // Fetch poll
    let poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1")
        .bind(poll_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

    // Fetch market
    let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1")
        .bind(poll_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    // Fetch options
    let options = sqlx::query_as::<_, PollOption>(
        r#"SELECT * FROM poll_options WHERE poll_id = $1 ORDER BY "order""#
    )
    .bind(poll_id)
    .fetch_all(db)
    .await?;

    // Parse shares from JSON
    let shares_map: std::collections::HashMap<String, f64> = 
        serde_json::from_value(market.shares_outstanding.clone()).unwrap_or_default();
    
    let shares: Vec<f64> = options
        .iter()
        .map(|opt| shares_map.get(&opt.id.to_string()).copied().unwrap_or(0.0))
        .collect();

    // Calculate current prices
    let option_responses: Vec<PollOptionResponse> = options
        .iter()
        .enumerate()
        .map(|(idx, opt)| PollOptionResponse {
            id: opt.id,
            text: opt.text.clone(),
            order: opt.order,
            price: lmsr::calculate_price(&shares, idx, market.liquidity_b),
        })
        .collect();

    // Fetch bet statistics
    let stats_row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_bets,
            COUNT(DISTINCT user_id) as unique_bettors,
            COALESCE(SUM(amount), 0) as total_volume
        FROM bets
        WHERE poll_id = $1
        "#
    )
    .bind(poll_id)
    .fetch_one(db)
    .await?;

    let stats = PollStats {
        total_bets: stats_row.try_get("total_bets")?,
        unique_bettors: stats_row.try_get("unique_bettors")?,
        total_volume: stats_row.try_get("total_volume")?,
    };

    // Fetch user position if authenticated
    let user_position = if let Some(Extension(AuthUser(claims))) = auth_user {
        let position = sqlx::query_as::<_, MarketPosition>(
            "SELECT * FROM market_positions WHERE user_id = $1 AND market_id = $2"
        )
        .bind(claims.sub)
        .bind(market.id)
        .fetch_optional(db)
        .await?;

        position.map(|pos| {
            let shares_map: std::collections::HashMap<String, f64> = 
                serde_json::from_value(pos.option_shares).unwrap_or_default();
            let spent_map: std::collections::HashMap<String, f64> = 
                serde_json::from_value(pos.option_spent).unwrap_or_default();

            let shares: std::collections::HashMap<i64, f64> = shares_map
                .into_iter()
                .filter_map(|(k, v)| k.parse::<i64>().ok().map(|id| (id, v)))
                .collect();

            let spent: std::collections::HashMap<i64, f64> = spent_map
                .into_iter()
                .filter_map(|(k, v)| k.parse::<i64>().ok().map(|id| (id, v)))
                .collect();

            UserPosition { shares, spent }
        })
    } else {
        None
    };

    let poll_detail = PollDetail {
        id: poll.id,
        title: poll.title,
        description: poll.description,
        status: poll.status,
        closes_at: poll.closes_at,
        created_at: poll.created_at,
        creator_id: poll.creator_id,
        category_id: poll.category_id,
        is_free: poll.is_free,
        resolution_criteria: poll.resolution_criteria,
        winning_option_id: poll.winning_option_id,
        options: option_responses,
        market: MarketResponse {
            id: market.id,
            liquidity_b: market.liquidity_b,
        },
        stats,
        user_position: user_position.clone(),
    };

    // Cache poll details if no user-specific data (fallback on error)
    if user_position.is_none() {
        if let Ok(json) = serde_json::to_string(&poll_detail) {
            let _ = crate::services::cache::cache_poll(&state, poll_id, &json).await;
        }
    }

    Ok(Json(poll_detail))
}


#[derive(Debug, Deserialize)]
pub struct ResolvePollRequest {
    pub winning_option_id: i64,
}

#[derive(Debug, Serialize)]
pub struct ResolutionResult {
    pub winners_count: i64,
    pub total_payout: f64,
}

/// POST /api/v1/polls/:id/resolve - Resolve a poll (admin only)
pub async fn resolve_poll(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(poll_id): Path<i64>,
    Json(req): Json<ResolvePollRequest>,
) -> AppResult<Json<ResolutionResult>> {
    let db = state.db();
    let user_id = claims.sub;

    // Check if user is admin
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if !user.is_staff {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    // Start transaction
    let mut tx = db.begin().await?;

    // Lock poll
    let mut poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1 FOR UPDATE")
        .bind(poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

    // Verify poll is closed
    if poll.status != "closed" {
        return Err(AppError::BadRequest(format!("Poll must be closed before resolution. Current status: {}", poll.status)));
    }

    // Verify closes_at has passed
    if poll.closes_at > Utc::now() {
        return Err(AppError::BadRequest("Poll has not closed yet".to_string()));
    }

    // Verify winning option belongs to poll
    let _option = sqlx::query_as::<_, PollOption>(
        "SELECT * FROM poll_options WHERE id = $1 AND poll_id = $2"
    )
    .bind(req.winning_option_id)
    .bind(poll_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid winning_option_id".to_string()))?;

    // Update poll
    poll.winning_option_id = Some(req.winning_option_id);
    poll.status = "resolved".to_string();

    sqlx::query("UPDATE polls SET winning_option_id = $1, status = $2 WHERE id = $3")
        .bind(req.winning_option_id)
        .bind(&poll.status)
        .bind(poll_id)
        .execute(&mut *tx)
        .await?;

    // Fetch market
    let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1")
        .bind(poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    // Query all positions with shares in winning option
    let positions = sqlx::query_as::<_, MarketPosition>(
        "SELECT * FROM market_positions WHERE market_id = $1"
    )
    .bind(market.id)
    .fetch_all(&mut *tx)
    .await?;

    let mut winners_count = 0;
    let mut total_payout = 0.0;

    for pos in positions {
        let shares_map: HashMap<String, f64> =
            serde_json::from_value(pos.option_shares).unwrap_or_default();

        let winning_shares = shares_map
            .get(&req.winning_option_id.to_string())
            .copied()
            .unwrap_or(0.0);

        if winning_shares > 0.0 {
            // Payout is shares * 1.0 (winning shares are worth 1.0 each)
            let payout = winning_shares;

            // Credit user balance
            sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
                .bind(payout)
                .bind(pos.user_id)
                .execute(&mut *tx)
                .await?;

            // Get updated balance
            let user_balance: f64 = sqlx::query_scalar("SELECT balance FROM users WHERE id = $1")
                .bind(pos.user_id)
                .fetch_one(&mut *tx)
                .await?;

            // Record wallet transaction
            sqlx::query(
                r#"
                INSERT INTO wallet_transactions (user_id, amount, transaction_type, balance_after, description, related_poll_id)
                VALUES ($1, $2, 'win', $3, $4, $5)
                "#
            )
            .bind(pos.user_id)
            .bind(payout)
            .bind(user_balance)
            .bind(format!("Won bet on poll: {}", poll.title))
            .bind(poll_id)
            .execute(&mut *tx)
            .await?;

            // Create notification
            sqlx::query(
                r#"
                INSERT INTO notifications (user_id, notification_type, message, is_read)
                VALUES ($1, 'bet_won', $2, false)
                "#
            )
            .bind(pos.user_id)
            .bind(format!("You won {} on '{}'!", payout, poll.title))
            .execute(&mut *tx)
            .await?;

            winners_count += 1;
            total_payout += payout;
        }
    }

    // TODO: Update user profile statistics (streaks, accuracy)

    // Commit transaction
    tx.commit().await?;

    // Invalidate caches (fallback on error)
    let _ = crate::services::cache::invalidate_poll_cache(&state, poll_id).await;
    let _ = crate::services::cache::invalidate_market_prices_cache(&state, poll_id).await;

    // Broadcast resolution event via WebSocket
    crate::ws::broadcast_poll_resolved(&state, poll_id, req.winning_option_id).await;

    Ok(Json(ResolutionResult {
        winners_count,
        total_payout,
    }))
}


#[derive(Debug, Serialize)]
pub struct StatusMessage {
    pub message: String,
}

/// POST /api/v1/polls/:id/suspend - Suspend a poll (admin only)
pub async fn suspend_poll(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(poll_id): Path<i64>,
) -> AppResult<Json<StatusMessage>> {
    let db = state.db();

    // Check if user is admin
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(claims.sub)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if !user.is_staff {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    // Update poll status
    let result = sqlx::query("UPDATE polls SET status = 'suspended' WHERE id = $1")
        .bind(poll_id)
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Poll not found".to_string()));
    }

    // Invalidate caches (fallback on error)
    let _ = crate::services::cache::invalidate_poll_cache(&state, poll_id).await;
    let _ = crate::services::cache::invalidate_market_prices_cache(&state, poll_id).await;

    Ok(Json(StatusMessage {
        message: "Poll suspended successfully".to_string(),
    }))
}


/// POST /api/v1/polls/:id/cancel - Cancel a poll with refunds (admin only)
pub async fn cancel_poll(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(poll_id): Path<i64>,
) -> AppResult<Json<StatusMessage>> {
    let db = state.db();

    // Check if user is admin
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(claims.sub)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if !user.is_staff {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    // Start transaction
    let mut tx = db.begin().await?;

    // Lock poll
    let poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1 FOR UPDATE")
        .bind(poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

    // Update poll status
    sqlx::query("UPDATE polls SET status = 'cancelled' WHERE id = $1")
        .bind(poll_id)
        .execute(&mut *tx)
        .await?;

    // Fetch all bets for this poll
    let bets = sqlx::query_as::<_, Bet>("SELECT * FROM bets WHERE poll_id = $1")
        .bind(poll_id)
        .fetch_all(&mut *tx)
        .await?;

    let bet_count = bets.len();

    // Refund each bet
    for bet in bets {
        // Credit user balance
        sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
            .bind(bet.amount)
            .bind(bet.user_id)
            .execute(&mut *tx)
            .await?;

        // Get updated balance
        let user_balance: f64 = sqlx::query_scalar("SELECT balance FROM users WHERE id = $1")
            .bind(bet.user_id)
            .fetch_one(&mut *tx)
            .await?;

        // Record wallet transaction
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (user_id, amount, transaction_type, balance_after, description, related_poll_id, related_bet_id)
            VALUES ($1, $2, 'refund', $3, $4, $5, $6)
            "#
        )
        .bind(bet.user_id)
        .bind(bet.amount)
        .bind(user_balance)
        .bind(format!("Refund for cancelled poll: {}", poll.title))
        .bind(poll_id)
        .bind(bet.id)
        .execute(&mut *tx)
        .await?;

        // Create notification
        sqlx::query(
            r#"
            INSERT INTO notifications (user_id, notification_type, message, is_read)
            VALUES ($1, 'bet_refunded', $2, false)
            "#
        )
        .bind(bet.user_id)
        .bind(format!("Poll '{}' was cancelled. Your bet of {} has been refunded.", poll.title, bet.amount))
        .execute(&mut *tx)
        .await?;
    }

    // Fetch market
    let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1")
        .bind(poll_id)
        .fetch_optional(&mut *tx)
        .await?;

    if let Some(market) = market {
        // Reset all market positions to zero
        sqlx::query(
            "UPDATE market_positions SET option_shares = '{}', option_spent = '{}' WHERE market_id = $1"
        )
        .bind(market.id)
        .execute(&mut *tx)
        .await?;
    }

    // Commit transaction
    tx.commit().await?;

    // Invalidate caches (fallback on error)
    let _ = crate::services::cache::invalidate_poll_cache(&state, poll_id).await;
    let _ = crate::services::cache::invalidate_market_prices_cache(&state, poll_id).await;

    Ok(Json(StatusMessage {
        message: format!("Poll cancelled successfully. Refunded {} bets.", bet_count),
    }))
}
