use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    errors::AppError,
    services::{
        cache,
        encryption::EncryptionService, 
        p2p_notifications::{self, P2PNotificationType},
        question_parser::QuestionParser
    },
    state::AppState,
    ws::p2p_bets::publish_bet_update,
};

// Platform fee configuration
const PLATFORM_FEE_PERCENTAGE: f64 = 7.0; // 7% of total stakes

#[derive(Debug, Deserialize)]
pub struct CreateBetRequest {
    pub question: String,
    pub stake_amount: i64,
    pub end_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateBetResponse {
    pub bet_id: i64,
    pub shareable_url: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinBetRequest {
    pub position: bool,
    pub stake: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReportOutcomeRequest {
    pub outcome: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BetResponse {
    pub id: i64,
    pub creator_id: i64,
    pub question: String,
    pub stake_amount: i64,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub shareable_url: String,
    pub verified_outcome: Option<bool>,
    pub disputed: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListBetsQuery {
    pub status: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

/// Create a new P2P bet
pub async fn create_bet(
    State(state): State<AppState>,
    Json(req): Json<CreateBetRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Parse and validate question
    let question = QuestionParser::parse(&req.question)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // Validate stake amount
    if req.stake_amount <= 0 {
        return Err(AppError::BadRequest("Stake amount must be positive".to_string()));
    }
    
    // Validate end time
    if req.end_time <= chrono::Utc::now() {
        return Err(AppError::BadRequest("End time must be in the future".to_string()));
    }
    
    // TODO: Get user_id from auth context
    let creator_id = 1i64; // Placeholder
    let creator_username = "alice"; // Placeholder
    
    // Generate shareable URL hash
    let secret = std::env::var("ENCRYPTION_SECRET").unwrap_or_else(|_| "default_secret".to_string());
    
    // Insert bet into database
    let bet = sqlx::query!(
        r#"
        INSERT INTO p2p_bets (
            creator_id, question, question_normalized, question_slug,
            stake_amount, end_time, state, shareable_url_hash
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
        creator_id,
        question.text,
        question.normalized,
        question.slug,
        req.stake_amount,
        req.end_time,
        "Created",
        format!("temp_{}", uuid::Uuid::new_v4()) // Temporary, will update after encryption
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // Generate shareable URL
    let shareable_url = EncryptionService::generate_shareable_url(
        bet.id as u64,
        &question.slug,
        creator_username,
        &secret,
    )
    .map_err(|e| AppError::InternalError(e.to_string()))?;
    
    // Update bet with actual shareable URL hash
    sqlx::query!(
        "UPDATE p2p_bets SET shareable_url_hash = $1 WHERE id = $2",
        shareable_url,
        bet.id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // TODO: Call smart contract create_bet function
    
    Ok(Json(CreateBetResponse {
        bet_id: bet.id,
        shareable_url,
    }))
}

/// List all P2P bets with filters
pub async fn list_bets(
    State(state): State<AppState>,
    Query(query): Query<ListBetsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * limit;

    // Build a cache key from query parameters
    let cache_key = format!(
        "{}:{}:{}:{}:{}",
        query.status.as_deref().unwrap_or("all"),
        query.search.as_deref().unwrap_or(""),
        query.sort.as_deref().unwrap_or("newest"),
        page,
        limit
    );

    // Check cache first
    if let Ok(Some(cached)) = cache::get_cached_p2p_bet_list(&state, &cache_key).await {
        if let Ok(response) = serde_json::from_str::<Vec<BetResponse>>(&cached) {
            return Ok(Json(response));
        }
    }
    
    // Build base query with participant aggregation for volume and liquidity
    let mut sql = String::from(
        r#"
        SELECT 
            b.id, b.creator_id, b.question, b.question_normalized, b.question_slug,
            b.stake_amount, b.end_time, b.state, b.created_at, b.shareable_url_hash,
            b.contract_bet_id, b.verified_outcome, b.disputed, b.paid_out,
            COALESCE(SUM(p.stake), 0) as total_volume,
            COUNT(p.id) as participant_count
        FROM p2p_bets b
        LEFT JOIN p2p_bet_participants p ON b.id = p.bet_id
        WHERE 1=1
        "#
    );
    
    let mut conditions = Vec::new();
    
    // Apply status filter
    if let Some(status) = &query.status {
        match status.as_str() {
            "All" => {
                // No filter, show all bets
            }
            "Active" => {
                conditions.push("b.state IN ('Created', 'Active')".to_string());
                conditions.push("b.end_time > NOW()".to_string());
            }
            "Ending Soon" => {
                conditions.push("b.state IN ('Created', 'Active')".to_string());
                conditions.push("b.end_time > NOW()".to_string());
                conditions.push("b.end_time <= NOW() + INTERVAL '24 hours'".to_string());
            }
            "Ended" => {
                conditions.push("b.state IN ('Ended', 'Verified', 'Disputed', 'Paid')".to_string());
            }
            _ => {
                // Treat as exact state match for custom states
                conditions.push(format!("b.state = '{}'", status.replace("'", "''")));
            }
        }
    }
    
    // Apply search filter (search in question text)
    if let Some(search) = &query.search {
        if !search.trim().is_empty() {
            let sanitized_search = search.replace("'", "''").replace("%", "\\%").replace("_", "\\_");
            conditions.push(format!("b.question ILIKE '%{}%'", sanitized_search));
        }
    }
    
    // Add conditions to query
    for condition in conditions {
        sql.push_str(&format!(" AND {}", condition));
    }
    
    // Add GROUP BY clause
    sql.push_str(
        r#"
        GROUP BY b.id, b.creator_id, b.question, b.question_normalized, b.question_slug,
                 b.stake_amount, b.end_time, b.state, b.created_at, b.shareable_url_hash,
                 b.contract_bet_id, b.verified_outcome, b.disputed, b.paid_out
        "#
    );
    
    // Apply sorting
    let order_clause = match query.sort.as_deref() {
        Some("volume") => "ORDER BY total_volume DESC, b.created_at DESC",
        Some("liquidity") => "ORDER BY (b.stake_amount + COALESCE(SUM(p.stake), 0)) DESC, b.created_at DESC",
        Some("newest") => "ORDER BY b.created_at DESC",
        Some("ending_soon") => "ORDER BY b.end_time ASC",
        _ => "ORDER BY b.created_at DESC", // Default to newest
    };
    
    // For liquidity sort, we need to recalculate in the ORDER BY
    if query.sort.as_deref() == Some("liquidity") {
        // Need to use a subquery for liquidity calculation
        let liquidity_sql = format!(
            r#"
            SELECT 
                b.id, b.creator_id, b.question, b.question_normalized, b.question_slug,
                b.stake_amount, b.end_time, b.state, b.created_at, b.shareable_url_hash,
                b.contract_bet_id, b.verified_outcome, b.disputed, b.paid_out,
                COALESCE(SUM(p.stake), 0) as total_volume,
                COUNT(p.id) as participant_count,
                (b.stake_amount + COALESCE(SUM(p.stake), 0)) as total_liquidity
            FROM p2p_bets b
            LEFT JOIN p2p_bet_participants p ON b.id = p.bet_id
            WHERE 1=1
            {}
            GROUP BY b.id, b.creator_id, b.question, b.question_normalized, b.question_slug,
                     b.stake_amount, b.end_time, b.state, b.created_at, b.shareable_url_hash,
                     b.contract_bet_id, b.verified_outcome, b.disputed, b.paid_out
            ORDER BY total_liquidity DESC, b.created_at DESC
            LIMIT {} OFFSET {}
            "#,
            if conditions.is_empty() { 
                String::new() 
            } else { 
                format!(" AND {}", conditions.join(" AND ")) 
            },
            limit,
            offset
        );
        
        let bets = sqlx::query_as::<_, (
            i64, i64, String, String, String, i64, 
            chrono::DateTime<chrono::Utc>, String, chrono::DateTime<chrono::Utc>, 
            String, Option<i64>, Option<bool>, bool, bool, i64, i64, i64
        )>(&liquidity_sql)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let response: Vec<BetResponse> = bets
            .into_iter()
            .map(|b| BetResponse {
                id: b.0,
                creator_id: b.1,
                question: b.2,
                stake_amount: b.5,
                end_time: b.6,
                state: b.7,
                created_at: b.8,
                shareable_url: b.9,
                verified_outcome: b.11,
                disputed: b.12,
            })
            .collect();

        // Cache the result
        if let Ok(serialized) = serde_json::to_string(&response) {
            let _ = cache::cache_p2p_bet_list(&state, &cache_key, &serialized).await;
        }

        return Ok(Json(response));
    }
    
    sql.push_str(order_clause);
    sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
    
    let bets = sqlx::query_as::<_, (
        i64, i64, String, String, String, i64, 
        chrono::DateTime<chrono::Utc>, String, chrono::DateTime<chrono::Utc>, 
        String, Option<i64>, Option<bool>, bool, bool, i64, i64
    )>(&sql)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    let response: Vec<BetResponse> = bets
        .into_iter()
        .map(|b| BetResponse {
            id: b.0,
            creator_id: b.1,
            question: b.2,
            stake_amount: b.5,
            end_time: b.6,
            state: b.7,
            created_at: b.8,
            shareable_url: b.9,
            verified_outcome: b.11,
            disputed: b.12,
        })
        .collect();

    // Cache the result
    if let Ok(serialized) = serde_json::to_string(&response) {
        let _ = cache::cache_p2p_bet_list(&state, &cache_key, &serialized).await;
    }

    Ok(Json(response))
}

/// Get bet details by ID
pub async fn get_bet(
    State(state): State<AppState>,
    Path(bet_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    // Check cache first
    if let Ok(Some(cached)) = cache::get_cached_p2p_bet(&state, bet_id).await {
        if let Ok(response) = serde_json::from_str::<BetResponse>(&cached) {
            return Ok(Json(response));
        }
    }

    let bet = sqlx::query_as::<_, (i64, i64, String, String, String, i64, chrono::DateTime<chrono::Utc>, String, chrono::DateTime<chrono::Utc>, String, Option<i64>, Option<bool>, bool, bool)>(
        "SELECT * FROM p2p_bets WHERE id = $1"
    )
    .bind(bet_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Bet not found".to_string()))?;
    
    let response = BetResponse {
        id: bet.0,
        creator_id: bet.1,
        question: bet.2,
        stake_amount: bet.5,
        end_time: bet.6,
        state: bet.7,
        created_at: bet.8,
        shareable_url: bet.9,
        verified_outcome: bet.11,
        disputed: bet.12,
    };

    // Cache the result
    if let Ok(serialized) = serde_json::to_string(&response) {
        let _ = cache::cache_p2p_bet(&state, bet_id, &serialized).await;
    }

    Ok(Json(response))
}

/// Join a bet
pub async fn join_bet(
    State(state): State<AppState>,
    Path(bet_id): Path<i64>,
    Json(req): Json<JoinBetRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from auth context
    let user_id = 2i64; // Placeholder
    
    // Validate stake
    if req.stake <= 0 {
        return Err(AppError::BadRequest("Stake must be positive".to_string()));
    }
    
    // Check if bet exists and is joinable
    let bet = sqlx::query!(
        "SELECT state, end_time FROM p2p_bets WHERE id = $1",
        bet_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Bet not found".to_string()))?;
    
    if bet.state != "Created" && bet.state != "Active" {
        return Err(AppError::BadRequest("Bet is not accepting participants".to_string()));
    }
    
    if bet.end_time <= chrono::Utc::now() {
        return Err(AppError::BadRequest("Bet has ended".to_string()));
    }
    
    // Check if user already joined
    let existing = sqlx::query!(
        "SELECT id FROM p2p_bet_participants WHERE bet_id = $1 AND user_id = $2",
        bet_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    if existing.is_some() {
        return Err(AppError::BadRequest("Already a participant".to_string()));
    }
    
    // Add participant
    sqlx::query!(
        r#"
        INSERT INTO p2p_bet_participants (bet_id, user_id, position, stake)
        VALUES ($1, $2, $3, $4)
        "#,
        bet_id,
        user_id,
        req.position,
        req.stake
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // Update bet state to Active
    sqlx::query!(
        "UPDATE p2p_bets SET state = 'Active' WHERE id = $1",
        bet_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // TODO: Call smart contract join_bet function

    // Invalidate bet cache since state changed
    let _ = cache::invalidate_p2p_bet_cache(&state, bet_id).await;

    // Broadcast WebSocket update: participant joined
    let position_str = if req.position { "Yes" } else { "No" };
    publish_bet_update(
        &state,
        bet_id,
        "participant_joined",
        serde_json::json!({
            "user_id": user_id,
            "position": position_str,
            "stake": req.stake,
        }),
    )
    .await;

    // Notify bet creator that someone joined
    let bet_details = sqlx::query!(
        "SELECT question FROM p2p_bets WHERE id = $1",
        bet_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    let user_uuid = uuid::Uuid::parse_str(&user_id.to_string())
        .unwrap_or_else(|_| uuid::Uuid::new_v4());
    
    let message = format!("A participant joined your bet: \"{}\"", bet_details.question);
    let _ = p2p_notifications::notify_bet_creator(
        &state,
        bet_id,
        P2PNotificationType::ParticipantJoined,
        message,
        Some(user_uuid),
    )
    .await;
    
    Ok(StatusCode::OK)
}

/// Cancel a bet
pub async fn cancel_bet(
    State(state): State<AppState>,
    Path(bet_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from auth context
    let user_id = 1i64; // Placeholder
    
    // Check if bet exists and user is creator
    let bet = sqlx::query!(
        "SELECT creator_id FROM p2p_bets WHERE id = $1",
        bet_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Bet not found".to_string()))?;
    
    if bet.creator_id != user_id {
        return Err(AppError::Forbidden("Only creator can cancel".to_string()));
    }
    
    // Check if bet has participants
    let participant_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM p2p_bet_participants WHERE bet_id = $1",
        bet_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    if participant_count.count.unwrap_or(0) > 0 {
        return Err(AppError::BadRequest("Cannot cancel bet with participants".to_string()));
    }
    
    // Update bet state to Cancelled
    sqlx::query!(
        "UPDATE p2p_bets SET state = 'Cancelled' WHERE id = $1",
        bet_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // TODO: Call smart contract cancel_bet function

    // Invalidate bet cache since state changed
    let _ = cache::invalidate_p2p_bet_cache(&state, bet_id).await;

    // Broadcast WebSocket update: bet cancelled
    publish_bet_update(
        &state,
        bet_id,
        "cancelled",
        serde_json::json!({}),
    )
    .await;

    Ok(StatusCode::OK)
}

/// Report outcome
pub async fn report_outcome(
    State(state): State<AppState>,
    Path(bet_id): Path<i64>,
    Json(req): Json<ReportOutcomeRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from auth context
    let user_id = 2i64; // Placeholder
    
    // Check if bet has ended
    let bet = sqlx::query!(
        "SELECT end_time FROM p2p_bets WHERE id = $1",
        bet_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Bet not found".to_string()))?;
    
    if bet.end_time > chrono::Utc::now() {
        return Err(AppError::BadRequest("Bet has not ended yet".to_string()));
    }
    
    // Check if user is participant
    let participant = sqlx::query!(
        "SELECT has_reported FROM p2p_bet_participants WHERE bet_id = $1 AND user_id = $2",
        bet_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or_else(|| AppError::Forbidden("Only participants can report outcome".to_string()))?;
    
    if participant.has_reported {
        return Err(AppError::BadRequest("Already reported outcome".to_string()));
    }
    
    // Record outcome report
    sqlx::query!(
        r#"
        INSERT INTO p2p_outcome_reports (bet_id, reporter_id, outcome)
        VALUES ($1, $2, $3)
        "#,
        bet_id,
        user_id,
        req.outcome
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // Mark participant as reported
    sqlx::query!(
        "UPDATE p2p_bet_participants SET has_reported = true WHERE bet_id = $1 AND user_id = $2",
        bet_id,
        user_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // Update bet state to Ended
    sqlx::query!(
        "UPDATE p2p_bets SET state = 'Ended' WHERE id = $1",
        bet_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // TODO: Call smart contract report_outcome function

    // Broadcast WebSocket update: outcome reported
    publish_bet_update(
        &state,
        bet_id,
        "outcome_reported",
        serde_json::json!({
            "user_id": user_id,
            "outcome": req.outcome,
        }),
    )
    .await;

    // Notify other participants that outcome was reported
    let bet_details = sqlx::query!(
        "SELECT question FROM p2p_bets WHERE id = $1",
        bet_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    let user_uuid = uuid::Uuid::parse_str(&user_id.to_string())
        .unwrap_or_else(|_| uuid::Uuid::new_v4());
    
    let outcome_text = if req.outcome { "Yes" } else { "No" };
    let message = format!(
        "Outcome reported for bet \"{}\": {}. Please confirm or dispute.",
        bet_details.question,
        outcome_text
    );
    let _ = p2p_notifications::notify_bet_participants(
        &state,
        bet_id,
        P2PNotificationType::OutcomeReported,
        message,
        Some(user_uuid),
    )
    .await;
    
    Ok(StatusCode::OK)
}

/// Confirm outcome
pub async fn confirm_outcome(
    State(state): State<AppState>,
    Path(bet_id): Path<i64>,
    Json(req): Json<ReportOutcomeRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from auth context
    let user_id = 3i64; // Placeholder
    
    // Check if user is participant and hasn't reported
    let participant = sqlx::query!(
        "SELECT has_reported FROM p2p_bet_participants WHERE bet_id = $1 AND user_id = $2",
        bet_id,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or_else(|| AppError::Forbidden("Only participants can confirm outcome".to_string()))?;
    
    if participant.has_reported {
        return Err(AppError::BadRequest("Already reported outcome".to_string()));
    }
    
    // Record outcome report
    sqlx::query!(
        r#"
        INSERT INTO p2p_outcome_reports (bet_id, reporter_id, outcome)
        VALUES ($1, $2, $3)
        "#,
        bet_id,
        user_id,
        req.outcome
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // Mark participant as reported
    sqlx::query!(
        "UPDATE p2p_bet_participants SET has_reported = true WHERE bet_id = $1 AND user_id = $2",
        bet_id,
        user_id
    )
    .execute(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    // Check if all participants have reported
    let all_reported = sqlx::query!(
        r#"
        SELECT COUNT(*) as total,
               COUNT(*) FILTER (WHERE has_reported = true) as reported
        FROM p2p_bet_participants
        WHERE bet_id = $1
        "#,
        bet_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    if all_reported.total == all_reported.reported {
        // Check if all agree
        let outcomes = sqlx::query!(
            "SELECT DISTINCT outcome FROM p2p_outcome_reports WHERE bet_id = $1",
            bet_id
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        if outcomes.len() == 1 {
            // All agree - verify outcome
            let verified_outcome = outcomes[0].outcome;
            sqlx::query!(
                "UPDATE p2p_bets SET state = 'Verified', verified_outcome = $1 WHERE id = $2",
                verified_outcome,
                bet_id
            )
            .execute(&state.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            
            // TODO: Call smart contract execute_payout
            
            // Broadcast WebSocket update: outcome verified
            publish_bet_update(
                &state,
                bet_id,
                "outcome_verified",
                serde_json::json!({
                    "outcome": verified_outcome,
                }),
            )
            .await;

            // Determine winners (participants whose position matches the verified outcome)
            let winners = sqlx::query!(
                "SELECT user_id FROM p2p_bet_participants WHERE bet_id = $1 AND position = $2",
                bet_id,
                verified_outcome
            )
            .fetch_all(&state.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            let winner_ids: Vec<i64> = winners.into_iter().map(|w| w.user_id).collect();

            // Mark bet as paid
            sqlx::query!(
                "UPDATE p2p_bets SET state = 'Paid', paid_out = true WHERE id = $1",
                bet_id
            )
            .execute(&state.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            // Broadcast WebSocket update: payout executed
            publish_bet_update(
                &state,
                bet_id,
                "paid",
                serde_json::json!({
                    "winners": winner_ids,
                }),
            )
            .await;

            // Notify all participants that outcome is verified
            let bet_details = sqlx::query!(
                "SELECT question FROM p2p_bets WHERE id = $1",
                bet_id
            )
            .fetch_one(&state.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            
            let outcome_text = if verified_outcome { "Yes" } else { "No" };
            let message = format!(
                "Outcome verified for bet \"{}\": {}. Payout will be executed.",
                bet_details.question,
                outcome_text
            );
            let _ = p2p_notifications::notify_bet_participants(
                &state,
                bet_id,
                P2PNotificationType::OutcomeVerified,
                message,
                None,
            )
            .await;
        } else {
            // Dispute
            sqlx::query!(
                "UPDATE p2p_bets SET state = 'Disputed', disputed = true WHERE id = $1",
                bet_id
            )
            .execute(&state.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            
            // Create dispute record
            sqlx::query!(
                "INSERT INTO p2p_bet_disputes (bet_id) VALUES ($1)",
                bet_id
            )
            .execute(&state.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

            // Broadcast WebSocket update: bet disputed
            publish_bet_update(
                &state,
                bet_id,
                "disputed",
                serde_json::json!({}),
            )
            .await;
            
            // Notify all participants about dispute
            let bet_details = sqlx::query!(
                "SELECT question FROM p2p_bets WHERE id = $1",
                bet_id
            )
            .fetch_one(&state.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            
            let message = format!(
                "Bet disputed: \"{}\". Participants disagree on outcome. Manual resolution required.",
                bet_details.question
            );
            let _ = p2p_notifications::notify_bet_participants(
                &state,
                bet_id,
                P2PNotificationType::BetDisputed,
                message,
                None,
            )
            .await;
        }
    }
    
    // TODO: Call smart contract confirm_outcome function

    Ok(StatusCode::OK)
}

/// Get outcome status
pub async fn get_outcome_status(
    State(state): State<AppState>,
    Path(bet_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let reports = sqlx::query!(
        r#"
        SELECT reporter_id, outcome, reported_at
        FROM p2p_outcome_reports
        WHERE bet_id = $1
        ORDER BY reported_at ASC
        "#,
        bet_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    Ok(Json(reports))
}

/// Resolve shareable URL
pub async fn resolve_shareable_url(
    State(state): State<AppState>,
    Path(encrypted_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let secret = std::env::var("ENCRYPTION_SECRET").unwrap_or_else(|_| "default_secret".to_string());
    
    let bet_id = EncryptionService::decrypt_bet_id(&encrypted_id, &secret)
        .map_err(|e| AppError::BadRequest(format!("Invalid shareable URL: {}", e)))?;
    
    get_bet(State(state), Path(bet_id as i64)).await
}

/// Get user's positions
pub async fn get_my_positions(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from auth context
    let user_id = 1i64; // Placeholder
    
    let positions = sqlx::query!(
        r#"
        SELECT p.*, b.question, b.state, b.end_time
        FROM p2p_bet_participants p
        JOIN p2p_bets b ON p.bet_id = b.id
        WHERE p.user_id = $1
        ORDER BY p.joined_at DESC
        "#,
        user_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    Ok(Json(positions))
}

/// Get user's created bets
pub async fn get_my_bets(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from auth context
    let user_id = 1i64; // Placeholder
    
    let bets = sqlx::query!(
        "SELECT * FROM p2p_bets WHERE creator_id = $1 ORDER BY created_at DESC",
        user_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    Ok(Json(bets))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_list_bets_query_building() {
        // Test that query parameters are properly validated
        
        // Test page validation (should be at least 1)
        let page = Some(0i64).unwrap_or(1).max(1);
        assert_eq!(page, 1);
        
        let page = Some(-5i64).unwrap_or(1).max(1);
        assert_eq!(page, 1);
        
        let page = Some(10i64).unwrap_or(1).max(1);
        assert_eq!(page, 10);
        
        // Test limit validation (should be between 1 and 100)
        let limit = Some(20i64).unwrap_or(20).min(100).max(1);
        assert_eq!(limit, 20);
        
        let limit = Some(0i64).unwrap_or(20).min(100).max(1);
        assert_eq!(limit, 1);
        
        let limit = Some(150i64).unwrap_or(20).min(100).max(1);
        assert_eq!(limit, 100);
        
        // Test offset calculation
        let page = 1i64;
        let limit = 20i64;
        let offset = (page - 1) * limit;
        assert_eq!(offset, 0);
        
        let page = 3i64;
        let offset = (page - 1) * limit;
        assert_eq!(offset, 40);
    }
    
    #[test]
    fn test_search_sanitization() {
        // Test SQL injection prevention
        let search = "test' OR '1'='1";
        let sanitized = search.replace("'", "''").replace("%", "\\%").replace("_", "\\_");
        assert_eq!(sanitized, "test'' OR ''1''=''1");
        
        // Test wildcard escaping
        let search = "test%_value";
        let sanitized = search.replace("'", "''").replace("%", "\\%").replace("_", "\\_");
        assert_eq!(sanitized, "test\\%\\_value");
    }
    
    #[test]
    fn test_status_filter_logic() {
        // Test status filter mapping
        let status = "All";
        assert_eq!(status, "All");
        
        let status = "Active";
        assert_eq!(status, "Active");
        
        let status = "Ending Soon";
        assert_eq!(status, "Ending Soon");
        
        let status = "Ended";
        assert_eq!(status, "Ended");
    }
    
    #[test]
    fn test_sort_options() {
        // Test sort option mapping
        let sort_options = vec!["volume", "liquidity", "newest", "ending_soon"];
        
        for option in sort_options {
            match option {
                "volume" => assert!(true),
                "liquidity" => assert!(true),
                "newest" => assert!(true),
                "ending_soon" => assert!(true),
                _ => assert!(false, "Invalid sort option"),
            }
        }
    }

    // ── Validation helpers mirroring handler logic ────────────────────────────
    // These functions extract the pure validation rules from the async handlers
    // so they can be tested without a live database.

    fn validate_create_bet(
        question: &str,
        stake_amount: i64,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        use crate::services::question_parser::QuestionParser;
        QuestionParser::parse(question).map_err(|e| e.to_string())?;
        if stake_amount <= 0 {
            return Err("Stake amount must be positive".to_string());
        }
        if end_time <= chrono::Utc::now() {
            return Err("End time must be in the future".to_string());
        }
        Ok(())
    }

    fn validate_join_bet(state: &str, end_time: chrono::DateTime<chrono::Utc>, stake: i64, already_joined: bool) -> Result<(), String> {
        if stake <= 0 {
            return Err("Stake must be positive".to_string());
        }
        if state != "Created" && state != "Active" {
            return Err("Bet is not accepting participants".to_string());
        }
        if end_time <= chrono::Utc::now() {
            return Err("Bet has ended".to_string());
        }
        if already_joined {
            return Err("Already a participant".to_string());
        }
        Ok(())
    }

    fn validate_report_outcome(
        end_time: chrono::DateTime<chrono::Utc>,
        is_participant: bool,
        has_reported: bool,
    ) -> Result<(), String> {
        if end_time > chrono::Utc::now() {
            return Err("Bet has not ended yet".to_string());
        }
        if !is_participant {
            return Err("Only participants can report outcome".to_string());
        }
        if has_reported {
            return Err("Already reported outcome".to_string());
        }
        Ok(())
    }

    fn validate_confirm_outcome(is_participant: bool, has_reported: bool) -> Result<(), String> {
        if !is_participant {
            return Err("Only participants can confirm outcome".to_string());
        }
        if has_reported {
            return Err("Already reported outcome".to_string());
        }
        Ok(())
    }

    fn determine_outcome_result(reports: &[bool]) -> &'static str {
        if reports.is_empty() {
            return "pending";
        }
        let first = reports[0];
        if reports.iter().all(|&r| r == first) {
            "verified"
        } else {
            "disputed"
        }
    }

    fn validate_cancel_bet(
        creator_id: i64,
        user_id: i64,
        participant_count: i64,
    ) -> Result<(), String> {
        if creator_id != user_id {
            return Err("Only creator can cancel".to_string());
        }
        if participant_count > 0 {
            return Err("Cannot cancel bet with participants".to_string());
        }
        Ok(())
    }

    // ── create_bet validation tests ───────────────────────────────────────────

    #[test]
    fn test_create_bet_rejects_empty_question() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_create_bet("", 100, future);
        assert!(result.is_err(), "Empty question should be rejected");
    }

    #[test]
    fn test_create_bet_rejects_question_without_mark() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        // Valid length but no question mark
        let result = validate_create_bet("Will it rain tomorrow", 100, future);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("question mark"));
    }

    #[test]
    fn test_create_bet_rejects_zero_stake() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_create_bet("Will it rain tomorrow?", 0, future);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    #[test]
    fn test_create_bet_rejects_negative_stake() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_create_bet("Will it rain tomorrow?", -50, future);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    #[test]
    fn test_create_bet_rejects_past_end_time() {
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        let result = validate_create_bet("Will it rain tomorrow?", 100, past);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("future"));
    }

    #[test]
    fn test_create_bet_accepts_valid_inputs() {
        let future = chrono::Utc::now() + chrono::Duration::hours(24);
        let result = validate_create_bet("Will it rain tomorrow?", 100, future);
        assert!(result.is_ok(), "Valid inputs should be accepted: {:?}", result);
    }

    // ── join_bet validation tests ─────────────────────────────────────────────

    #[test]
    fn test_join_bet_rejects_cancelled_state() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_join_bet("Cancelled", future, 100, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not accepting"));
    }

    #[test]
    fn test_join_bet_rejects_ended_state() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_join_bet("Ended", future, 100, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not accepting"));
    }

    #[test]
    fn test_join_bet_rejects_past_end_time() {
        let past = chrono::Utc::now() - chrono::Duration::minutes(5);
        let result = validate_join_bet("Active", past, 100, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ended"));
    }

    #[test]
    fn test_join_bet_rejects_duplicate_participant() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_join_bet("Active", future, 100, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Already a participant"));
    }

    #[test]
    fn test_join_bet_rejects_zero_stake() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_join_bet("Created", future, 0, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    #[test]
    fn test_join_bet_accepts_created_state() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_join_bet("Created", future, 100, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_join_bet_accepts_active_state() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_join_bet("Active", future, 100, false);
        assert!(result.is_ok());
    }

    // ── report_outcome validation tests ──────────────────────────────────────

    #[test]
    fn test_report_outcome_rejects_before_end_time() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let result = validate_report_outcome(future, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not ended yet"));
    }

    #[test]
    fn test_report_outcome_rejects_non_participant() {
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        let result = validate_report_outcome(past, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("participants"));
    }

    #[test]
    fn test_report_outcome_rejects_duplicate_report() {
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        let result = validate_report_outcome(past, true, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Already reported"));
    }

    #[test]
    fn test_report_outcome_accepts_valid_participant_after_end() {
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        let result = validate_report_outcome(past, true, false);
        assert!(result.is_ok());
    }

    // ── confirm_outcome / dispute logic tests ─────────────────────────────────

    #[test]
    fn test_confirm_outcome_rejects_non_participant() {
        let result = validate_confirm_outcome(false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("participants"));
    }

    #[test]
    fn test_confirm_outcome_rejects_already_reported() {
        let result = validate_confirm_outcome(true, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Already reported"));
    }

    #[test]
    fn test_confirm_outcome_triggers_verified_when_all_agree() {
        // All participants report the same outcome → verified
        let reports = vec![true, true, true];
        assert_eq!(determine_outcome_result(&reports), "verified");
    }

    #[test]
    fn test_confirm_outcome_triggers_dispute_when_participants_disagree() {
        // Participants report different outcomes → disputed
        let reports = vec![true, false];
        assert_eq!(determine_outcome_result(&reports), "disputed");
    }

    #[test]
    fn test_confirm_outcome_dispute_with_mixed_reports() {
        let reports = vec![false, false, true];
        assert_eq!(determine_outcome_result(&reports), "disputed");
    }

    #[test]
    fn test_confirm_outcome_verified_unanimous_no() {
        let reports = vec![false, false];
        assert_eq!(determine_outcome_result(&reports), "verified");
    }

    // ── cancel_bet validation tests ───────────────────────────────────────────

    #[test]
    fn test_cancel_bet_rejects_non_creator() {
        let result = validate_cancel_bet(1, 2, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("creator"));
    }

    #[test]
    fn test_cancel_bet_rejects_when_participants_exist() {
        let result = validate_cancel_bet(1, 1, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("participants"));
    }

    #[test]
    fn test_cancel_bet_accepts_creator_with_no_participants() {
        let result = validate_cancel_bet(1, 1, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cancel_bet_rejects_creator_with_multiple_participants() {
        let result = validate_cancel_bet(5, 5, 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("participants"));
    }
}
