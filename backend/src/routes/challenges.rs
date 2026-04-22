use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::BigDecimal, Row};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Challenge, Poll, User},
    state::AppState,
};

// ─── Request/Response Types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateChallengeRequest {
    pub question: String,
    pub amount: f64,
    pub creator_choice: String,
    pub opponent_id: Option<Uuid>,
    pub is_open: bool,
    pub poll_id: Option<i64>,
    pub expires_at: DateTime<Utc>,
    pub resolution_criteria: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChallengeListParams {
    pub status: Option<String>,
    pub is_open: Option<bool>,
    pub creator_id: Option<Uuid>,
    pub opponent_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub id: i64,
    pub question: String,
    pub amount: String,
    pub creator_choice: String,
    pub status: String,
    pub is_open: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ChallengeListItem {
    pub id: i64,
    pub question: String,
    pub amount: String,
    pub creator_choice: String,
    pub status: String,
    pub is_open: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub creator: UserInfo,
    pub opponent: Option<UserInfo>,
}

#[derive(Debug, Serialize)]
pub struct ChallengeDetail {
    pub id: i64,
    pub question: String,
    pub amount: String,
    pub creator_choice: String,
    pub status: String,
    pub is_open: bool,
    pub expires_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub winner_id: Option<Uuid>,
    pub resolution_criteria: String,
    pub created_at: DateTime<Utc>,
    pub creator: UserInfo,
    pub opponent: Option<UserInfo>,
    pub poll: Option<PollInfo>,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct PollInfo {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub winning_option_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct StatusMessage {
    pub message: String,
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

/// POST /api/v1/challenges - Create a new challenge
pub async fn create_challenge(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Json(req): Json<CreateChallengeRequest>,
) -> AppResult<Json<ChallengeResponse>> {
    // Validate amount
    if req.amount < 1.0 {
        return Err(AppError::BadRequest("Minimum challenge amount is 1.0".to_string()));
    }

    let db = state.db();
    let user_id = claims.sub;

    // Check user balance
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if user.balance < req.amount {
        return Err(AppError::BadRequest(format!(
            "Insufficient balance. Have: {}, Need: {}",
            user.balance, req.amount
        )));
    }

    // If poll_id provided, verify poll exists and is open
    if let Some(poll_id) = req.poll_id {
        let poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1")
            .bind(poll_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

        if poll.status != "open" {
            return Err(AppError::BadRequest(format!("Poll is {}, cannot create challenge", poll.status)));
        }
    }

    // If direct challenge, verify opponent_id is provided
    if !req.is_open && req.opponent_id.is_none() {
        return Err(AppError::BadRequest("opponent_id required for direct challenges".to_string()));
    }

    // Start transaction
    let mut tx = db.begin().await?;

    // Insert challenge
    let amount_decimal = BigDecimal::from(req.amount as i64);

    let challenge = sqlx::query_as::<_, Challenge>(
        r#"
        INSERT INTO challenges (creator_id, opponent_id, question, amount, creator_choice, status, is_open, poll_id, expires_at, resolution_criteria)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(req.opponent_id)
    .bind(&req.question)
    .bind(amount_decimal)
    .bind(&req.creator_choice)
    .bind(req.is_open)
    .bind(req.poll_id)
    .bind(req.expires_at)
    .bind(req.resolution_criteria.unwrap_or_default())
    .fetch_one(&mut *tx)
    .await?;

    // If direct challenge, create notification for opponent
    if let Some(opponent_id) = req.opponent_id {
        let notification_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO notifications (user_id, actor_id, notification_type, message, is_read)
            VALUES ($1, $2, 'challenge_received', $3, false)
            RETURNING id
            "#
        )
        .bind(opponent_id)
        .bind(user_id)
        .bind(format!("{} challenged you to: {}", user.username, req.question))
        .fetch_one(&mut *tx)
        .await?;

        // Store notification_id and opponent_id for WebSocket broadcast after commit
        tx.commit().await?;

        // Send real-time notification via WebSocket
        crate::ws::send_notification_to_user(
            &state,
            opponent_id,
            notification_id,
            "challenge_received".to_string(),
            format!("{} challenged you to: {}", user.username, req.question),
            Some(user_id),
        ).await;

        // Send challenge invite event
        crate::ws::send_challenge_invite(
            &state,
            opponent_id,
            challenge.id,
            user_id,
            req.amount.to_string(),
            req.question.clone(),
        ).await;
    } else {
        tx.commit().await?;
    }

    Ok(Json(ChallengeResponse {
        id: challenge.id,
        question: challenge.question,
        amount: challenge.amount.to_string(),
        creator_choice: challenge.creator_choice,
        status: challenge.status,
        is_open: challenge.is_open,
        expires_at: challenge.expires_at,
        created_at: challenge.created_at,
    }))
}


/// POST /api/v1/challenges/:id/accept - Accept a challenge
pub async fn accept_challenge(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(challenge_id): Path<i64>,
) -> AppResult<Json<ChallengeResponse>> {
    let db = state.db();
    let user_id = claims.sub;

    // Start transaction
    let mut tx = db.begin().await?;

    // Lock challenge
    let mut challenge = sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges WHERE id = $1 FOR UPDATE"
    )
    .bind(challenge_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Challenge not found".to_string()))?;

    // Verify challenge status
    if challenge.status != "pending" {
        return Err(AppError::BadRequest(format!("Challenge is {}, cannot accept", challenge.status)));
    }

    // Verify not expired
    if challenge.expires_at <= Utc::now() {
        return Err(AppError::BadRequest("Challenge has expired".to_string()));
    }

    // Verify user is not the creator
    if challenge.creator_id == user_id {
        return Err(AppError::BadRequest("Cannot accept your own challenge".to_string()));
    }

    // Lock both users
    let creator = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
        .bind(challenge.creator_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Creator not found".to_string()))?;

    let acceptor = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let amount = challenge.amount.to_string().parse::<f64>()
        .map_err(|_| AppError::InternalServerError("Invalid amount".to_string()))?;

    // Check balances
    if creator.balance < amount {
        return Err(AppError::BadRequest("Creator has insufficient balance".to_string()));
    }
    if acceptor.balance < amount {
        return Err(AppError::BadRequest("You have insufficient balance".to_string()));
    }

    // Deduct from both users
    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
        .bind(amount)
        .bind(challenge.creator_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE users SET balance = balance - $1 WHERE id = $2")
        .bind(amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Get updated balances
    let creator_balance: f64 = sqlx::query_scalar("SELECT balance FROM users WHERE id = $1")
        .bind(challenge.creator_id)
        .fetch_one(&mut *tx)
        .await?;

    let acceptor_balance: f64 = sqlx::query_scalar("SELECT balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

    // Record wallet transactions
    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (user_id, amount, transaction_type, balance_after, description)
        VALUES ($1, $2, 'bet', $3, $4)
        "#
    )
    .bind(challenge.creator_id)
    .bind(-amount)
    .bind(creator_balance)
    .bind(format!("Challenge stake: {}", challenge.question))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (user_id, amount, transaction_type, balance_after, description)
        VALUES ($1, $2, 'bet', $3, $4)
        "#
    )
    .bind(user_id)
    .bind(-amount)
    .bind(acceptor_balance)
    .bind(format!("Challenge stake: {}", challenge.question))
    .execute(&mut *tx)
    .await?;

    // Update challenge
    challenge.status = "accepted".to_string();
    if challenge.is_open {
        challenge.opponent_id = Some(user_id);
    }

    sqlx::query("UPDATE challenges SET status = $1, opponent_id = $2 WHERE id = $3")
        .bind(&challenge.status)
        .bind(challenge.opponent_id)
        .bind(challenge_id)
        .execute(&mut *tx)
        .await?;

    // Create notification for creator
    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, actor_id, notification_type, message, is_read)
        VALUES ($1, $2, 'challenge_accepted', $3, false)
        "#
    )
    .bind(challenge.creator_id)
    .bind(user_id)
    .bind(format!("{} accepted your challenge: {}", acceptor.username, challenge.question))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ChallengeResponse {
        id: challenge.id,
        question: challenge.question,
        amount: challenge.amount.to_string(),
        creator_choice: challenge.creator_choice,
        status: challenge.status,
        is_open: challenge.is_open,
        expires_at: challenge.expires_at,
        created_at: challenge.created_at,
    }))
}


#[derive(Debug, Deserialize)]
pub struct ResolveChallengeRequest {
    pub winner_id: Option<Uuid>,
}

/// POST /api/v1/challenges/:id/resolve - Resolve a challenge
pub async fn resolve_challenge(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(challenge_id): Path<i64>,
    Json(req): Json<ResolveChallengeRequest>,
) -> AppResult<Json<ChallengeResponse>> {
    let db = state.db();
    let _user_id = claims.sub;

    // Start transaction
    let mut tx = db.begin().await?;

    // Lock challenge
    let mut challenge = sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges WHERE id = $1 FOR UPDATE"
    )
    .bind(challenge_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Challenge not found".to_string()))?;

    // Verify challenge status
    if challenge.status != "accepted" {
        return Err(AppError::BadRequest(format!("Challenge is {}, cannot resolve", challenge.status)));
    }

    // Determine winner
    let winner_id = if let Some(poll_id) = challenge.poll_id {
        // Resolve based on poll result
        let poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1")
            .bind(poll_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

        if poll.status != "resolved" {
            return Err(AppError::BadRequest("Poll must be resolved first".to_string()));
        }

        let winning_option_id = poll.winning_option_id
            .ok_or_else(|| AppError::BadRequest("Poll has no winning option".to_string()))?;

        let winning_option = sqlx::query_as::<_, crate::models::PollOption>(
            "SELECT * FROM poll_options WHERE id = $1"
        )
        .bind(winning_option_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Winning option not found".to_string()))?;

        // Check if creator's choice matches winning option
        if winning_option.text == challenge.creator_choice {
            challenge.creator_id
        } else {
            challenge.opponent_id
                .ok_or_else(|| AppError::BadRequest("No opponent for challenge".to_string()))?
        }
    } else {
        // Manual resolution
        req.winner_id
            .ok_or_else(|| AppError::BadRequest("winner_id required for manual resolution".to_string()))?
    };

    // Verify winner is one of the participants
    let opponent_id = challenge.opponent_id
        .ok_or_else(|| AppError::BadRequest("Challenge has no opponent".to_string()))?;

    if winner_id != challenge.creator_id && winner_id != opponent_id {
        return Err(AppError::BadRequest("winner_id must be creator or opponent".to_string()));
    }

    let amount = challenge.amount.to_string().parse::<f64>()
        .map_err(|_| AppError::InternalServerError("Invalid amount".to_string()))?;

    let payout = amount * 2.0;

    // Credit winner
    sqlx::query("UPDATE users SET balance = balance + $1 WHERE id = $2")
        .bind(payout)
        .bind(winner_id)
        .execute(&mut *tx)
        .await?;

    // Get updated balance
    let winner_balance: f64 = sqlx::query_scalar("SELECT balance FROM users WHERE id = $1")
        .bind(winner_id)
        .fetch_one(&mut *tx)
        .await?;

    // Record wallet transaction
    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (user_id, amount, transaction_type, balance_after, description)
        VALUES ($1, $2, 'win', $3, $4)
        "#
    )
    .bind(winner_id)
    .bind(payout)
    .bind(winner_balance)
    .bind(format!("Won challenge: {}", challenge.question))
    .execute(&mut *tx)
    .await?;

    // Update challenge
    challenge.status = "resolved".to_string();
    challenge.winner_id = Some(winner_id);
    challenge.resolved_at = Some(Utc::now());

    sqlx::query("UPDATE challenges SET status = $1, winner_id = $2, resolved_at = $3 WHERE id = $4")
        .bind(&challenge.status)
        .bind(winner_id)
        .bind(challenge.resolved_at)
        .bind(challenge_id)
        .execute(&mut *tx)
        .await?;

    // Create notifications for both participants
    let loser_id = if winner_id == challenge.creator_id {
        opponent_id
    } else {
        challenge.creator_id
    };

    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, notification_type, message, is_read)
        VALUES ($1, 'challenge_won', $2, false)
        "#
    )
    .bind(winner_id)
    .bind(format!("You won the challenge: {}! Prize: {}", challenge.question, payout))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, notification_type, message, is_read)
        VALUES ($1, 'challenge_lost', $2, false)
        "#
    )
    .bind(loser_id)
    .bind(format!("You lost the challenge: {}", challenge.question))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ChallengeResponse {
        id: challenge.id,
        question: challenge.question,
        amount: challenge.amount.to_string(),
        creator_choice: challenge.creator_choice,
        status: challenge.status,
        is_open: challenge.is_open,
        expires_at: challenge.expires_at,
        created_at: challenge.created_at,
    }))
}


/// GET /api/v1/challenges - List challenges with filters
pub async fn list_challenges(
    State(state): State<AppState>,
    Query(params): Query<ChallengeListParams>,
) -> AppResult<Json<Vec<ChallengeListItem>>> {
    let db = state.db();

    // Build query with filters
    let mut query = String::from(
        r#"
        SELECT 
            c.*,
            creator.id as creator_id, creator.username as creator_username,
            opponent.id as opponent_id, opponent.username as opponent_username
        FROM challenges c
        INNER JOIN users creator ON c.creator_id = creator.id
        LEFT JOIN users opponent ON c.opponent_id = opponent.id
        WHERE 1=1
        "#
    );

    let mut conditions = Vec::new();
    
    if let Some(status) = &params.status {
        conditions.push(format!("c.status = '{}'", status));
    }
    
    if let Some(is_open) = params.is_open {
        conditions.push(format!("c.is_open = {}", is_open));
    }
    
    if let Some(creator_id) = params.creator_id {
        conditions.push(format!("c.creator_id = '{}'", creator_id));
    }
    
    if let Some(opponent_id) = params.opponent_id {
        conditions.push(format!("c.opponent_id = '{}'", opponent_id));
    }

    for condition in conditions {
        query.push_str(&format!(" AND {}", condition));
    }

    query.push_str(" ORDER BY c.created_at DESC");

    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);
    query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

    // Execute query
    let rows = sqlx::query(&query)
        .fetch_all(db)
        .await?;

    let mut challenges = Vec::new();
    for row in rows {
        let opponent_id_opt: Option<Uuid> = row.get("opponent_id");
        
        let challenge = ChallengeListItem {
            id: row.get("id"),
            question: row.get("question"),
            amount: row.get::<BigDecimal, _>("amount").to_string(),
            creator_choice: row.get("creator_choice"),
            status: row.get("status"),
            is_open: row.get("is_open"),
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
            creator: UserInfo {
                id: row.get("creator_id"),
                username: row.get("creator_username"),
            },
            opponent: opponent_id_opt.map(|opponent_id| UserInfo {
                id: opponent_id,
                username: row.get("opponent_username"),
            }),
        };
        challenges.push(challenge);
    }

    Ok(Json(challenges))
}


/// GET /api/v1/challenges/:id - Get challenge detail
pub async fn get_challenge_detail(
    State(state): State<AppState>,
    Path(challenge_id): Path<i64>,
) -> AppResult<Json<ChallengeDetail>> {
    let db = state.db();

    // Query challenge with user details
    let row = sqlx::query(
        r#"
        SELECT 
            c.*,
            creator.id as creator_id, creator.username as creator_username,
            opponent.id as opponent_id, opponent.username as opponent_username
        FROM challenges c
        INNER JOIN users creator ON c.creator_id = creator.id
        LEFT JOIN users opponent ON c.opponent_id = opponent.id
        WHERE c.id = $1
        "#
    )
    .bind(challenge_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Challenge not found".to_string()))?;

    let poll_id: Option<i64> = row.get("poll_id");
    
    // If poll_id exists, fetch poll details
    let poll = if let Some(pid) = poll_id {
        let poll_row = sqlx::query_as::<_, Poll>(
            "SELECT * FROM polls WHERE id = $1"
        )
        .bind(pid)
        .fetch_optional(db)
        .await?;

        poll_row.map(|p| PollInfo {
            id: p.id,
            title: p.title,
            status: p.status,
            winning_option_id: p.winning_option_id,
        })
    } else {
        None
    };

    let opponent_id_opt: Option<Uuid> = row.get("opponent_id");

    let detail = ChallengeDetail {
        id: row.get("id"),
        question: row.get("question"),
        amount: row.get::<BigDecimal, _>("amount").to_string(),
        creator_choice: row.get("creator_choice"),
        status: row.get("status"),
        is_open: row.get("is_open"),
        expires_at: row.get("expires_at"),
        resolved_at: row.get("resolved_at"),
        winner_id: row.get("winner_id"),
        resolution_criteria: row.get("resolution_criteria"),
        created_at: row.get("created_at"),
        creator: UserInfo {
            id: row.get("creator_id"),
            username: row.get("creator_username"),
        },
        opponent: opponent_id_opt.map(|opponent_id| UserInfo {
            id: opponent_id,
            username: row.get("opponent_username"),
        }),
        poll,
    };

    Ok(Json(detail))
}


/// POST /api/v1/challenges/:id/cancel - Cancel a pending challenge
pub async fn cancel_challenge(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(challenge_id): Path<i64>,
) -> AppResult<Json<StatusMessage>> {
    let db = state.db();
    let user_id = claims.sub;

    // Start transaction
    let mut tx = db.begin().await?;

    // Lock challenge
    let challenge = sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges WHERE id = $1 FOR UPDATE"
    )
    .bind(challenge_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Challenge not found".to_string()))?;

    // Verify user is challenge creator
    if challenge.creator_id != user_id {
        return Err(AppError::Forbidden("Only the creator can cancel this challenge".to_string()));
    }

    // Verify challenge status is pending
    if challenge.status != "pending" {
        return Err(AppError::BadRequest(format!("Challenge is {}, cannot cancel", challenge.status)));
    }

    // Update challenge status to cancelled
    sqlx::query("UPDATE challenges SET status = 'cancelled' WHERE id = $1")
        .bind(challenge_id)
        .execute(&mut *tx)
        .await?;

    // If direct challenge, create notification for opponent
    if let Some(opponent_id) = challenge.opponent_id {
        let creator = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO notifications (user_id, actor_id, notification_type, message, is_read)
            VALUES ($1, $2, 'challenge_cancelled', $3, false)
            "#
        )
        .bind(opponent_id)
        .bind(user_id)
        .bind(format!("{} cancelled the challenge: {}", creator.username, challenge.question))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(StatusMessage {
        message: "Challenge cancelled successfully".to_string(),
    }))
}
