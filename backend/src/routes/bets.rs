use axum::{extract::State, Extension, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    errors::{AppError, AppResult},
    lmsr,
    middleware::auth::AuthUser,
    models::{Bet, Market, MarketPosition, Poll, PollOption, User},
    state::AppState,
};

// ─── Request/Response Types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PlaceBetRequest {
    pub poll_id: i64,
    pub option_id: i64,
    pub amount: f64,
}

#[derive(Debug, Serialize)]
pub struct BetResult {
    pub bet_id: i64,
    pub shares: f64,
    pub new_price: f64,
    pub balance_after: f64,
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

/// POST /api/v1/bets - Place a bet on a poll option
pub async fn place_bet(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Json(req): Json<PlaceBetRequest>,
) -> AppResult<Json<BetResult>> {
    // Validate amount
    if req.amount < 1.0 {
        return Err(AppError::BadRequest("Minimum bet amount is 1.0".to_string()));
    }

    let db = state.db();
    let user_id = claims.sub;

    // Start transaction
    let mut tx = db.begin().await?;

    // Lock user and check balance
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if user.balance < req.amount {
        return Err(AppError::BadRequest(format!(
            "Insufficient balance. Have: {}, Need: {}",
            user.balance, req.amount
        )));
    }

    // Lock poll and verify status
    let poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1 FOR UPDATE")
        .bind(req.poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

    if poll.status != "open" {
        return Err(AppError::Forbidden(format!("Poll is {}, cannot place bets", poll.status)));
    }

    if poll.closes_at <= Utc::now() {
        return Err(AppError::Forbidden("Poll has closed".to_string()));
    }

    // Lock market
    let mut market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1 FOR UPDATE")
        .bind(req.poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    // Fetch all options
    let options = sqlx::query_as::<_, PollOption>(
        r#"SELECT * FROM poll_options WHERE poll_id = $1 ORDER BY "order""#
    )
    .bind(req.poll_id)
    .fetch_all(&mut *tx)
    .await?;

    // Verify option exists
    let option_idx = options
        .iter()
        .position(|opt| opt.id == req.option_id)
        .ok_or_else(|| AppError::BadRequest("Invalid option_id".to_string()))?;

    // Parse current shares
    let shares_map: HashMap<String, f64> =
        serde_json::from_value(market.shares_outstanding.clone()).unwrap_or_default();

    let mut current_shares: Vec<f64> = options
        .iter()
        .map(|opt| shares_map.get(&opt.id.to_string()).copied().unwrap_or(0.0))
        .collect();

    // Calculate shares to issue using LMSR
    let shares_issued = lmsr::calculate_shares_for_cost(&current_shares, option_idx, req.amount, market.liquidity_b);

    // Update market shares
    current_shares[option_idx] += shares_issued;
    let mut new_shares_map = HashMap::new();
    for (idx, opt) in options.iter().enumerate() {
        new_shares_map.insert(opt.id.to_string(), current_shares[idx]);
    }
    market.shares_outstanding = serde_json::to_value(&new_shares_map)?;

    sqlx::query("UPDATE markets SET shares_outstanding = $1 WHERE id = $2")
        .bind(&market.shares_outstanding)
        .bind(market.id)
        .execute(&mut *tx)
        .await?;

    // Deduct balance
    let new_balance = user.balance - req.amount;
    sqlx::query("UPDATE users SET balance = $1 WHERE id = $2")
        .bind(new_balance)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Insert bet record
    let bet = sqlx::query_as::<_, Bet>(
        r#"
        INSERT INTO bets (user_id, poll_id, option_id, amount, shares)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(req.poll_id)
    .bind(req.option_id)
    .bind(req.amount)
    .bind(shares_issued)
    .fetch_one(&mut *tx)
    .await?;

    // Upsert market position
    let position = sqlx::query_as::<_, MarketPosition>(
        "SELECT * FROM market_positions WHERE user_id = $1 AND market_id = $2"
    )
    .bind(user_id)
    .bind(market.id)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(pos) = position {
        // Update existing position
        let mut shares_map: HashMap<String, f64> =
            serde_json::from_value(pos.option_shares).unwrap_or_default();
        let mut spent_map: HashMap<String, f64> =
            serde_json::from_value(pos.option_spent).unwrap_or_default();

        let option_key = req.option_id.to_string();
        *shares_map.entry(option_key.clone()).or_insert(0.0) += shares_issued;
        *spent_map.entry(option_key).or_insert(0.0) += req.amount;

        sqlx::query(
            "UPDATE market_positions SET option_shares = $1, option_spent = $2 WHERE id = $3"
        )
        .bind(serde_json::to_value(&shares_map)?)
        .bind(serde_json::to_value(&spent_map)?)
        .bind(pos.id)
        .execute(&mut *tx)
        .await?;
    } else {
        // Create new position
        let mut shares_map = HashMap::new();
        shares_map.insert(req.option_id.to_string(), shares_issued);

        let mut spent_map = HashMap::new();
        spent_map.insert(req.option_id.to_string(), req.amount);

        sqlx::query(
            r#"
            INSERT INTO market_positions (user_id, market_id, option_shares, option_spent)
            VALUES ($1, $2, $3, $4)
            "#
        )
        .bind(user_id)
        .bind(market.id)
        .bind(serde_json::to_value(&shares_map)?)
        .bind(serde_json::to_value(&spent_map)?)
        .execute(&mut *tx)
        .await?;
    }

    // Insert wallet transaction
    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (user_id, amount, transaction_type, balance_after, description, related_poll_id, related_bet_id)
        VALUES ($1, $2, 'bet', $3, $4, $5, $6)
        "#
    )
    .bind(user_id)
    .bind(-req.amount)
    .bind(new_balance)
    .bind(format!("Bet on poll: {}", poll.title))
    .bind(req.poll_id)
    .bind(bet.id)
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    // Calculate new price
    let new_price = lmsr::calculate_price(&current_shares, option_idx, market.liquidity_b);

    // Broadcast price update via WebSocket
    let mut prices = HashMap::new();
    for (idx, opt) in options.iter().enumerate() {
        let price = lmsr::calculate_price(&current_shares, idx, market.liquidity_b);
        prices.insert(opt.id.to_string(), price);
    }
    crate::ws::broadcast_price_update(&state, req.poll_id, prices).await;

    // Invalidate caches (fallback on error)
    let _ = crate::services::cache::invalidate_poll_cache(&state, req.poll_id).await;
    let _ = crate::services::cache::invalidate_market_prices_cache(&state, req.poll_id).await;

    Ok(Json(BetResult {
        bet_id: bet.id,
        shares: shares_issued,
        new_price,
        balance_after: new_balance,
    }))
}


#[derive(Debug, Deserialize)]
pub struct SellSharesRequest {
    pub poll_id: i64,
    pub option_id: i64,
    pub shares: f64,
}

#[derive(Debug, Serialize)]
pub struct SellResult {
    pub refund: f64,
    pub new_price: f64,
    pub balance_after: f64,
}

/// POST /api/v1/bets/sell - Sell shares for a poll option
pub async fn sell_shares(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Json(req): Json<SellSharesRequest>,
) -> AppResult<Json<SellResult>> {
    // Validate shares
    if req.shares <= 0.0 {
        return Err(AppError::BadRequest("Shares must be positive".to_string()));
    }

    let db = state.db();
    let user_id = claims.sub;

    // Start transaction
    let mut tx = db.begin().await?;

    // Lock user
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Lock poll
    let poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1 FOR UPDATE")
        .bind(req.poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

    // Lock market
    let mut market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1 FOR UPDATE")
        .bind(req.poll_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    // Fetch all options
    let options = sqlx::query_as::<_, PollOption>(
        r#"SELECT * FROM poll_options WHERE poll_id = $1 ORDER BY "order""#
    )
    .bind(req.poll_id)
    .fetch_all(&mut *tx)
    .await?;

    // Verify option exists
    let option_idx = options
        .iter()
        .position(|opt| opt.id == req.option_id)
        .ok_or_else(|| AppError::BadRequest("Invalid option_id".to_string()))?;

    // Get user's position
    let position = sqlx::query_as::<_, MarketPosition>(
        "SELECT * FROM market_positions WHERE user_id = $1 AND market_id = $2 FOR UPDATE"
    )
    .bind(user_id)
    .bind(market.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("No position found for this market".to_string()))?;

    // Parse user's shares
    let mut user_shares_map: HashMap<String, f64> =
        serde_json::from_value(position.option_shares.clone()).unwrap_or_default();
    let mut user_spent_map: HashMap<String, f64> =
        serde_json::from_value(position.option_spent.clone()).unwrap_or_default();

    let option_key = req.option_id.to_string();
    let user_shares = user_shares_map.get(&option_key).copied().unwrap_or(0.0);

    if user_shares < req.shares {
        return Err(AppError::BadRequest(format!(
            "Insufficient shares. Have: {}, Trying to sell: {}",
            user_shares, req.shares
        )));
    }

    // Parse current market shares
    let shares_map: HashMap<String, f64> =
        serde_json::from_value(market.shares_outstanding.clone()).unwrap_or_default();

    let mut current_shares: Vec<f64> = options
        .iter()
        .map(|opt| shares_map.get(&opt.id.to_string()).copied().unwrap_or(0.0))
        .collect();

    // Calculate refund using LMSR
    let refund = lmsr::calculate_refund(&current_shares, option_idx, req.shares, market.liquidity_b);

    // Update market shares
    current_shares[option_idx] = (current_shares[option_idx] - req.shares).max(0.0);
    let mut new_shares_map = HashMap::new();
    for (idx, opt) in options.iter().enumerate() {
        new_shares_map.insert(opt.id.to_string(), current_shares[idx]);
    }
    market.shares_outstanding = serde_json::to_value(&new_shares_map)?;

    sqlx::query("UPDATE markets SET shares_outstanding = $1 WHERE id = $2")
        .bind(&market.shares_outstanding)
        .bind(market.id)
        .execute(&mut *tx)
        .await?;

    // Credit user balance
    let new_balance = user.balance + refund;
    sqlx::query("UPDATE users SET balance = $1 WHERE id = $2")
        .bind(new_balance)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Update user position
    let new_user_shares = user_shares - req.shares;
    if new_user_shares > 0.0 {
        user_shares_map.insert(option_key.clone(), new_user_shares);
        
        // Proportionally reduce spent amount
        let user_spent = user_spent_map.get(&option_key).copied().unwrap_or(0.0);
        let new_spent = user_spent * (new_user_shares / user_shares);
        user_spent_map.insert(option_key, new_spent);
    } else {
        user_shares_map.remove(&option_key);
        user_spent_map.remove(&option_key);
    }

    sqlx::query(
        "UPDATE market_positions SET option_shares = $1, option_spent = $2 WHERE id = $3"
    )
    .bind(serde_json::to_value(&user_shares_map)?)
    .bind(serde_json::to_value(&user_spent_map)?)
    .bind(position.id)
    .execute(&mut *tx)
    .await?;

    // Insert wallet transaction
    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (user_id, amount, transaction_type, balance_after, description, related_poll_id)
        VALUES ($1, $2, 'refund', $3, $4, $5)
        "#
    )
    .bind(user_id)
    .bind(refund)
    .bind(new_balance)
    .bind(format!("Sold shares on poll: {}", poll.title))
    .bind(req.poll_id)
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    // Calculate new price
    let new_price = lmsr::calculate_price(&current_shares, option_idx, market.liquidity_b);

    // Broadcast price update via WebSocket
    let mut prices = HashMap::new();
    for (idx, opt) in options.iter().enumerate() {
        let price = lmsr::calculate_price(&current_shares, idx, market.liquidity_b);
        prices.insert(opt.id.to_string(), price);
    }
    crate::ws::broadcast_price_update(&state, req.poll_id, prices).await;

    // Invalidate caches (fallback on error)
    let _ = crate::services::cache::invalidate_poll_cache(&state, req.poll_id).await;
    let _ = crate::services::cache::invalidate_market_prices_cache(&state, req.poll_id).await;

    Ok(Json(SellResult {
        refund,
        new_price,
        balance_after: new_balance,
    }))
}


#[derive(Debug, Serialize)]
pub struct Position {
    pub poll_id: i64,
    pub poll_title: String,
    pub poll_status: String,
    pub closes_at: chrono::DateTime<Utc>,
    pub positions: Vec<OptionPosition>,
    pub total_spent: f64,
    pub current_value: f64,
    pub profit_loss: f64,
}

#[derive(Debug, Serialize)]
pub struct OptionPosition {
    pub option_id: i64,
    pub option_text: String,
    pub shares: f64,
    pub spent: f64,
    pub current_price: f64,
    pub current_value: f64,
}

/// GET /api/v1/positions - Get all positions for authenticated user
pub async fn get_user_positions(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
) -> AppResult<Json<Vec<Position>>> {
    let db = state.db();
    let user_id = claims.sub;

    // Fetch all market positions for user
    let positions = sqlx::query_as::<_, MarketPosition>(
        "SELECT * FROM market_positions WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    let mut result = Vec::new();

    for pos in positions {
        // Parse shares and spent
        let shares_map: HashMap<String, f64> =
            serde_json::from_value(pos.option_shares).unwrap_or_default();
        let spent_map: HashMap<String, f64> =
            serde_json::from_value(pos.option_spent).unwrap_or_default();

        // Skip if no shares
        if shares_map.values().all(|&v| v == 0.0) {
            continue;
        }

        // Fetch market
        let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE id = $1")
            .bind(pos.market_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

        // Fetch poll
        let poll = sqlx::query_as::<_, Poll>("SELECT * FROM polls WHERE id = $1")
            .bind(market.poll_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Poll not found".to_string()))?;

        // Fetch options
        let options = sqlx::query_as::<_, PollOption>(
            r#"SELECT * FROM poll_options WHERE poll_id = $1 ORDER BY "order""#
        )
        .bind(poll.id)
        .fetch_all(db)
        .await?;

        // Parse market shares
        let market_shares_map: HashMap<String, f64> =
            serde_json::from_value(market.shares_outstanding).unwrap_or_default();

        let market_shares: Vec<f64> = options
            .iter()
            .map(|opt| market_shares_map.get(&opt.id.to_string()).copied().unwrap_or(0.0))
            .collect();

        // Build option positions
        let mut option_positions = Vec::new();
        let mut total_spent = 0.0;
        let mut current_value = 0.0;

        for (idx, opt) in options.iter().enumerate() {
            let opt_key = opt.id.to_string();
            let shares = shares_map.get(&opt_key).copied().unwrap_or(0.0);

            if shares > 0.0 {
                let spent = spent_map.get(&opt_key).copied().unwrap_or(0.0);
                let current_price = lmsr::calculate_price(&market_shares, idx, market.liquidity_b);
                let value = shares * current_price;

                total_spent += spent;
                current_value += value;

                option_positions.push(OptionPosition {
                    option_id: opt.id,
                    option_text: opt.text.clone(),
                    shares,
                    spent,
                    current_price,
                    current_value: value,
                });
            }
        }

        result.push(Position {
            poll_id: poll.id,
            poll_title: poll.title,
            poll_status: poll.status,
            closes_at: poll.closes_at,
            positions: option_positions,
            total_spent,
            current_value,
            profit_loss: current_value - total_spent,
        });
    }

    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, state::AppState};
    use sqlx::PgPool;
    use uuid::Uuid;

    // Helper function to create test AppState
    async fn create_test_state(pool: PgPool) -> AppState {
        let config = Config {
            database_url: "".to_string(), // Not used in tests
            redis_url: "redis://localhost:6379".to_string(),
            jwt_secret: "test_secret".to_string(),
            cors_origins: vec!["http://localhost:3000".to_string()],
            rust_log: "info".to_string(),
            port: 8080,
            frontend_url: "http://localhost:3000".to_string(),
            web3auth_client_id: None,
            mpesa_env: None,
            mpesa_consumer_key: None,
            mpesa_consumer_secret: None,
            mpesa_shortcode: None,
            mpesa_passkey: None,
            mpesa_callback_url: None,
            email_host: None,
            email_port: 587,
            email_user: None,
            email_password: None,
            email_from: "test@example.com".to_string(),
        };

        // Create a minimal redis pool for testing
        let redis_cfg = deadpool_redis::Config::from_url(&config.redis_url);
        let redis = redis_cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)).unwrap();

        let connection_registry = crate::ws::ConnectionRegistry::new();
        let broadcast_hub = crate::ws::BroadcastHub::new(connection_registry.clone());
        let market_hub = crate::ws::MarketHub::new(broadcast_hub.clone());

        AppState {
            inner: std::sync::Arc::new(crate::state::Inner {
                db: pool,
                redis,
                config,
            }),
            market_hub,
            connection_registry,
            broadcast_hub,
        }
    }

    // Helper function to create test user
    async fn create_test_user(pool: &PgPool, balance: f64) -> Uuid {
        let user_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, balance, is_staff, is_active)
            VALUES ($1, $2, $3, $4, false, true)
            "#
        )
        .bind(user_id)
        .bind(format!("testuser_{}", user_id))
        .bind(format!("test_{}@example.com", user_id))
        .bind(balance)
        .execute(pool)
        .await
        .unwrap();

        user_id
    }

    // Helper function to create test poll with market
    async fn create_test_poll(pool: &PgPool, creator_id: Uuid) -> (i64, Vec<i64>) {
        // Create poll
        let poll = sqlx::query_as::<_, Poll>(
            r#"
            INSERT INTO polls (creator_id, title, description, status, is_free, closes_at, resolution_criteria)
            VALUES ($1, 'Test Poll', 'Test Description', 'open', true, NOW() + INTERVAL '1 day', 'Test criteria')
            RETURNING *
            "#
        )
        .bind(creator_id)
        .fetch_one(pool)
        .await
        .unwrap();

        // Create options
        let option1 = sqlx::query_as::<_, PollOption>(
            r#"
            INSERT INTO poll_options (poll_id, text, is_yes, "order")
            VALUES ($1, 'Yes', true, 0)
            RETURNING *
            "#
        )
        .bind(poll.id)
        .fetch_one(pool)
        .await
        .unwrap();

        let option2 = sqlx::query_as::<_, PollOption>(
            r#"
            INSERT INTO poll_options (poll_id, text, is_yes, "order")
            VALUES ($1, 'No', false, 1)
            RETURNING *
            "#
        )
        .bind(poll.id)
        .fetch_one(pool)
        .await
        .unwrap();

        // Create market
        let initial_shares = serde_json::json!({
            option1.id.to_string(): 0.0,
            option2.id.to_string(): 0.0,
        });

        sqlx::query(
            r#"
            INSERT INTO markets (poll_id, liquidity_b, shares_outstanding)
            VALUES ($1, 100.0, $2)
            "#
        )
        .bind(poll.id)
        .bind(&initial_shares)
        .execute(pool)
        .await
        .unwrap();

        (poll.id, vec![option1.id, option2.id])
    }

    #[sqlx::test]
    async fn test_successful_bet_placement(pool: PgPool) {
        // Setup
        let user_id = create_test_user(&pool, 100.0).await;
        let (poll_id, option_ids) = create_test_poll(&pool, user_id).await;

        // Create request
        let req = PlaceBetRequest {
            poll_id,
            option_id: option_ids[0],
            amount: 10.0,
        };

        // Create state
        let state = create_test_state(pool.clone()).await;

        // Create auth claims
        let claims = crate::middleware::auth::Claims {
            sub: user_id,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            email: format!("test_{}@example.com", user_id),
        };

        // Execute
        let result = place_bet(
            State(state),
            Extension(AuthUser(claims)),
            Json(req),
        )
        .await;

        // Assert
        assert!(result.is_ok(), "Bet placement should succeed");
        let bet_result = result.unwrap().0;
        assert!(bet_result.shares > 0.0, "Should receive shares");
        assert_eq!(bet_result.balance_after, 90.0, "Balance should be deducted");
        assert!(bet_result.new_price > 0.0 && bet_result.new_price < 1.0, "Price should be valid");

        // Verify database state
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user.balance, 90.0, "User balance should be updated");

        // Verify bet record
        let bet_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(bet_count, 1, "Bet record should be created");

        // Verify wallet transaction
        let tx_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM wallet_transactions WHERE user_id = $1 AND transaction_type = 'bet'"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(tx_count, 1, "Wallet transaction should be recorded");
    }

    #[sqlx::test]
    async fn test_insufficient_balance_rejection(pool: PgPool) {
        // Setup - user with only 5.0 balance
        let user_id = create_test_user(&pool, 5.0).await;
        let (poll_id, option_ids) = create_test_poll(&pool, user_id).await;

        // Create request with amount > balance
        let req = PlaceBetRequest {
            poll_id,
            option_id: option_ids[0],
            amount: 10.0,
        };

        let state = create_test_state(pool.clone()).await;
        let claims = crate::middleware::auth::Claims {
            sub: user_id,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            email: format!("test_{}@example.com", user_id),
        };

        // Execute
        let result = place_bet(
            State(state),
            Extension(AuthUser(claims)),
            Json(req),
        )
        .await;

        // Assert
        assert!(result.is_err(), "Should reject insufficient balance");
        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("Insufficient balance"), "Error message should mention insufficient balance");
            }
            _ => panic!("Expected BadRequest error"),
        }

        // Verify no changes to database
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user.balance, 5.0, "Balance should remain unchanged");

        let bet_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(bet_count, 0, "No bet should be created");
    }

    #[sqlx::test]
    async fn test_closed_market_rejection(pool: PgPool) {
        // Setup
        let user_id = create_test_user(&pool, 100.0).await;
        
        // Create closed poll
        let poll = sqlx::query_as::<_, Poll>(
            r#"
            INSERT INTO polls (creator_id, title, description, status, is_free, closes_at, resolution_criteria)
            VALUES ($1, 'Closed Poll', 'Test Description', 'closed', true, NOW() - INTERVAL '1 day', 'Test criteria')
            RETURNING *
            "#
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Create option
        let option = sqlx::query_as::<_, PollOption>(
            r#"
            INSERT INTO poll_options (poll_id, text, is_yes, "order")
            VALUES ($1, 'Yes', true, 0)
            RETURNING *
            "#
        )
        .bind(poll.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Create market
        let initial_shares = serde_json::json!({
            option.id.to_string(): 0.0,
        });

        sqlx::query(
            r#"
            INSERT INTO markets (poll_id, liquidity_b, shares_outstanding)
            VALUES ($1, 100.0, $2)
            "#
        )
        .bind(poll.id)
        .bind(&initial_shares)
        .execute(&pool)
        .await
        .unwrap();

        // Create request
        let req = PlaceBetRequest {
            poll_id: poll.id,
            option_id: option.id,
            amount: 10.0,
        };

        let state = create_test_state(pool.clone()).await;
        let claims = crate::middleware::auth::Claims {
            sub: user_id,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            email: format!("test_{}@example.com", user_id),
        };

        // Execute
        let result = place_bet(
            State(state),
            Extension(AuthUser(claims)),
            Json(req),
        )
        .await;

        // Assert
        assert!(result.is_err(), "Should reject bet on closed market");
        match result.unwrap_err() {
            AppError::Forbidden(msg) => {
                assert!(msg.contains("closed"), "Error message should mention closed status");
            }
            _ => panic!("Expected Forbidden error"),
        }

        // Verify no bet was created
        let bet_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(bet_count, 0, "No bet should be created");
    }

    #[sqlx::test]
    async fn test_minimum_bet_amount_enforcement(pool: PgPool) {
        // Setup
        let user_id = create_test_user(&pool, 100.0).await;
        let (poll_id, option_ids) = create_test_poll(&pool, user_id).await;

        // Create request with amount < 1.0
        let req = PlaceBetRequest {
            poll_id,
            option_id: option_ids[0],
            amount: 0.5,
        };

        let state = create_test_state(pool.clone()).await;
        let claims = crate::middleware::auth::Claims {
            sub: user_id,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            email: format!("test_{}@example.com", user_id),
        };

        // Execute
        let result = place_bet(
            State(state),
            Extension(AuthUser(claims)),
            Json(req),
        )
        .await;

        // Assert
        assert!(result.is_err(), "Should reject bet below minimum amount");
        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("Minimum bet amount"), "Error message should mention minimum amount");
            }
            _ => panic!("Expected BadRequest error"),
        }

        // Verify no bet was created
        let bet_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(bet_count, 0, "No bet should be created");
    }

    #[sqlx::test]
    async fn test_transaction_rollback_on_error(pool: PgPool) {
        // Setup
        let user_id = create_test_user(&pool, 100.0).await;
        let (poll_id, _option_ids) = create_test_poll(&pool, user_id).await;

        // Create request with invalid option_id to trigger error mid-transaction
        let req = PlaceBetRequest {
            poll_id,
            option_id: 99999, // Non-existent option
            amount: 10.0,
        };

        let state = create_test_state(pool.clone()).await;
        let claims = crate::middleware::auth::Claims {
            sub: user_id,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            email: format!("test_{}@example.com", user_id),
        };

        // Execute
        let result = place_bet(
            State(state),
            Extension(AuthUser(claims)),
            Json(req),
        )
        .await;

        // Assert
        assert!(result.is_err(), "Should fail with invalid option");

        // Verify rollback - balance should remain unchanged
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user.balance, 100.0, "Balance should not be deducted on error");

        // Verify no bet was created
        let bet_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(bet_count, 0, "No bet should be created on error");

        // Verify no wallet transaction was created
        let tx_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM wallet_transactions WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(tx_count, 0, "No wallet transaction should be created on error");

        // Verify market shares remain unchanged
        let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1")
            .bind(poll_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        
        let shares_map: HashMap<String, f64> =
            serde_json::from_value(market.shares_outstanding).unwrap();
        
        // All shares should still be 0.0 (initial state)
        for shares in shares_map.values() {
            assert_eq!(*shares, 0.0, "Market shares should remain at initial state");
        }
    }

    #[sqlx::test]
    async fn test_multiple_bets_on_same_option(pool: PgPool) {
        // Setup
        let user_id = create_test_user(&pool, 100.0).await;
        let (poll_id, option_ids) = create_test_poll(&pool, user_id).await;

        let state = create_test_state(pool.clone()).await;
        let claims = crate::middleware::auth::Claims {
            sub: user_id,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            email: format!("test_{}@example.com", user_id),
        };

        // Place first bet
        let req1 = PlaceBetRequest {
            poll_id,
            option_id: option_ids[0],
            amount: 10.0,
        };

        let result1 = place_bet(
            State(state.clone()),
            Extension(AuthUser(claims.clone())),
            Json(req1),
        )
        .await;
        assert!(result1.is_ok(), "First bet should succeed");

        // Place second bet on same option
        let req2 = PlaceBetRequest {
            poll_id,
            option_id: option_ids[0],
            amount: 15.0,
        };

        let result2 = place_bet(
            State(state),
            Extension(AuthUser(claims)),
            Json(req2),
        )
        .await;
        assert!(result2.is_ok(), "Second bet should succeed");

        // Verify total balance deduction
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user.balance, 75.0, "Balance should reflect both bets");

        // Verify both bets recorded
        let bet_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bets WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(bet_count, 2, "Both bets should be recorded");

        // Verify position aggregates both bets
        let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1")
            .bind(poll_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let position = sqlx::query_as::<_, MarketPosition>(
            "SELECT * FROM market_positions WHERE user_id = $1 AND market_id = $2"
        )
        .bind(user_id)
        .bind(market.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let spent_map: HashMap<String, f64> =
            serde_json::from_value(position.option_spent).unwrap();
        let total_spent = spent_map.get(&option_ids[0].to_string()).copied().unwrap_or(0.0);
        assert_eq!(total_spent, 25.0, "Position should aggregate both bets");
    }

    #[sqlx::test]
    async fn test_bet_on_different_options(pool: PgPool) {
        // Setup
        let user_id = create_test_user(&pool, 100.0).await;
        let (poll_id, option_ids) = create_test_poll(&pool, user_id).await;

        let state = create_test_state(pool.clone()).await;
        let claims = crate::middleware::auth::Claims {
            sub: user_id,
            iat: chrono::Utc::now().timestamp(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
            email: format!("test_{}@example.com", user_id),
        };

        // Place bet on first option
        let req1 = PlaceBetRequest {
            poll_id,
            option_id: option_ids[0],
            amount: 20.0,
        };

        let result1 = place_bet(
            State(state.clone()),
            Extension(AuthUser(claims.clone())),
            Json(req1),
        )
        .await;
        assert!(result1.is_ok(), "First bet should succeed");
        let price1 = result1.unwrap().0.new_price;

        // Place bet on second option
        let req2 = PlaceBetRequest {
            poll_id,
            option_id: option_ids[1],
            amount: 20.0,
        };

        let result2 = place_bet(
            State(state),
            Extension(AuthUser(claims)),
            Json(req2),
        )
        .await;
        assert!(result2.is_ok(), "Second bet should succeed");
        let price2 = result2.unwrap().0.new_price;

        // Verify prices are balanced (should be close to 0.5 each)
        assert!((price1 - 0.5).abs() < 0.2, "Prices should be balanced after equal bets");
        assert!((price2 - 0.5).abs() < 0.2, "Prices should be balanced after equal bets");

        // Verify position has both options
        let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1")
            .bind(poll_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let position = sqlx::query_as::<_, MarketPosition>(
            "SELECT * FROM market_positions WHERE user_id = $1 AND market_id = $2"
        )
        .bind(user_id)
        .bind(market.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let shares_map: HashMap<String, f64> =
            serde_json::from_value(position.option_shares).unwrap();
        
        assert!(shares_map.contains_key(&option_ids[0].to_string()), "Should have shares in option 1");
        assert!(shares_map.contains_key(&option_ids[1].to_string()), "Should have shares in option 2");
    }
}
