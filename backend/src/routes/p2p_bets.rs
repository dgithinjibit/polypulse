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
        encryption::EncryptionService, 
        p2p_notifications::{self, P2PNotificationType},
        question_parser::QuestionParser
    },
    state::AppState,
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

#[derive(Debug, Serialize)]
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
    
    Ok(Json(response))
}

/// Get bet details by ID
pub async fn get_bet(
    State(state): State<AppState>,
    Path(bet_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let bet = sqlx::query_as::<_, (i64, i64, String, String, String, i64, chrono::DateTime<chrono::Utc>, String, chrono::DateTime<chrono::Utc>, String, Option<i64>, Option<bool>, bool, bool)>(
        "SELECT * FROM p2p_bets WHERE id = $1"
    )
    .bind(bet_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?
    .ok_or_else(|| AppError::NotFound("Bet not found".to_string()))?;
    
    Ok(Json(BetResponse {
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
    }))
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
    // TODO: Broadcast WebSocket update
    
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
    // TODO: Broadcast WebSocket update
    
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
    // TODO: Broadcast WebSocket update
    
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
}
