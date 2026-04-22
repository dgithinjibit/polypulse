//! Paymaster service for gasless transactions.
//!
//! Sponsors gas fees for user transactions on Secret Network, enabling
//! zero-friction onboarding. Implements retry logic, rate limiting, and
//! expenditure tracking.

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::state::AppState;

/// Maximum retry attempts for failed transactions.
const MAX_RETRIES: u32 = 3;

/// Rate limit: max transactions per user per hour.
const RATE_LIMIT_PER_HOUR: u32 = 10;

/// Redis key prefix for rate limiting.
const RATE_LIMIT_PREFIX: &str = "paymaster:ratelimit:";

/// Redis key prefix for gas expenditure tracking.
const GAS_TRACKING_PREFIX: &str = "paymaster:gas:";

/// Transaction status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TxStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
}

/// Gasless transaction record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaslessTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tx_hash: Option<String>,
    pub status: TxStatus,
    pub retry_count: u32,
    pub gas_used: Option<u64>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl GaslessTransaction {
    pub fn new(user_id: Uuid) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4(),
            user_id,
            tx_hash: None,
            status: TxStatus::Pending,
            retry_count: 0,
            gas_used: None,
            error: None,
            created_at: now,
            updated_at: now,
        }
    }
}

// ---------------------------------------------------------------------------
// Rate limiting
// ---------------------------------------------------------------------------

/// Check if user has exceeded the rate limit for gasless transactions.
pub async fn check_rate_limit(state: &AppState, user_id: &Uuid) -> Result<bool> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("{RATE_LIMIT_PREFIX}{user_id}");
    let count: Option<u32> = conn.get(&key).await?;

    match count {
        None => Ok(true), // No record, allow
        Some(c) if c < RATE_LIMIT_PER_HOUR => Ok(true),
        Some(_) => Ok(false), // Rate limit exceeded
    }
}

/// Increment the rate limit counter for a user.
async fn increment_rate_limit(state: &AppState, user_id: &Uuid) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("{RATE_LIMIT_PREFIX}{user_id}");
    let count: Option<u32> = conn.get(&key).await?;

    match count {
        None => {
            // First transaction in this hour
            let _: () = conn.set_ex(&key, 1u32, 3600).await?;
        }
        Some(c) => {
            // Increment existing counter
            let _: () = conn.set(&key, c + 1).await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Transaction relay
// ---------------------------------------------------------------------------

/// Submit a gasless transaction to Secret Network.
///
/// This is a placeholder implementation. In production, this would:
/// 1. Sign the transaction with the paymaster's private key
/// 2. Submit to Secret Network via LCD/RPC
/// 3. Return the transaction hash
pub async fn relay_transaction(
    state: &AppState,
    user_id: Uuid,
    tx_data: &[u8],
) -> Result<GaslessTransaction> {
    // Check rate limit
    if !check_rate_limit(state, &user_id).await? {
        return Err(anyhow!("Rate limit exceeded for user {user_id}"));
    }

    let mut tx = GaslessTransaction::new(user_id);

    // Attempt submission with retries
    for attempt in 1..=MAX_RETRIES {
        tx.retry_count = attempt - 1;
        tx.updated_at = Utc::now().timestamp();

        match submit_to_network(state, tx_data).await {
            Ok((tx_hash, gas_used)) => {
                tx.tx_hash = Some(tx_hash.clone());
                tx.status = TxStatus::Confirmed;
                tx.gas_used = Some(gas_used);

                info!(
                    "Gasless transaction confirmed for user {user_id}: {tx_hash} (gas: {gas_used})"
                );

                // Track gas expenditure
                track_gas_expenditure(state, user_id, gas_used).await?;

                // Increment rate limit counter
                increment_rate_limit(state, &user_id).await?;

                // Store transaction record in DB
                store_transaction(state, &tx).await?;

                return Ok(tx);
            }
            Err(e) => {
                warn!(
                    "Gasless transaction attempt {attempt}/{MAX_RETRIES} failed for user {user_id}: {e}"
                );
                tx.error = Some(e.to_string());

                if attempt == MAX_RETRIES {
                    tx.status = TxStatus::Failed;
                    store_transaction(state, &tx).await?;
                    return Err(anyhow!(
                        "Transaction failed after {MAX_RETRIES} attempts: {e}"
                    ));
                }

                // Exponential backoff
                tokio::time::sleep(tokio::time::Duration::from_millis(100 * 2u64.pow(attempt)))
                    .await;
            }
        }
    }

    Err(anyhow!("Transaction failed after all retries"))
}

/// Submit transaction to Secret Network (placeholder).
///
/// In production, this would use the Secret Network SDK to:
/// 1. Build a signed transaction with the paymaster's key
/// 2. Submit via LCD or RPC
/// 3. Wait for confirmation
/// 4. Return tx hash and gas used
async fn submit_to_network(_state: &AppState, _tx_data: &[u8]) -> Result<(String, u64)> {
    // Placeholder: simulate network submission
    debug!("Submitting transaction to Secret Network...");

    // Simulate network delay
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Simulate success (90% success rate for testing)
    if rand::random::<f32>() < 0.9 {
        let tx_hash = format!("0x{}", hex::encode(rand::random::<[u8; 32]>()));
        let gas_used = 50000 + rand::random::<u64>() % 50000; // 50k-100k gas
        Ok((tx_hash, gas_used))
    } else {
        Err(anyhow!("Network error: transaction rejected"))
    }
}

// ---------------------------------------------------------------------------
// Gas expenditure tracking
// ---------------------------------------------------------------------------

/// Track total gas expenditure for monitoring and budgeting.
async fn track_gas_expenditure(state: &AppState, user_id: Uuid, gas_used: u64) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    // Track per-user expenditure
    let user_key = format!("{GAS_TRACKING_PREFIX}user:{user_id}");
    let _: () = conn.incr(&user_key, gas_used).await?;

    // Track global expenditure
    let global_key = format!("{GAS_TRACKING_PREFIX}total");
    let _: () = conn.incr(&global_key, gas_used).await?;

    debug!("Tracked {gas_used} gas for user {user_id}");
    Ok(())
}

/// Get total gas expenditure for a user.
pub async fn get_user_gas_expenditure(state: &AppState, user_id: &Uuid) -> Result<u64> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("{GAS_TRACKING_PREFIX}user:{user_id}");
    let total: Option<u64> = conn.get(&key).await?;
    Ok(total.unwrap_or(0))
}

/// Get total platform gas expenditure.
pub async fn get_total_gas_expenditure(state: &AppState) -> Result<u64> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let key = format!("{GAS_TRACKING_PREFIX}total");
    let total: Option<u64> = conn.get(&key).await?;
    Ok(total.unwrap_or(0))
}

// ---------------------------------------------------------------------------
// Database persistence
// ---------------------------------------------------------------------------

/// Store transaction record in PostgreSQL.
async fn store_transaction(state: &AppState, tx: &GaslessTransaction) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO gasless_transactions (id, user_id, tx_hash, status, retry_count, gas_used, error, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8), to_timestamp($9))
        ON CONFLICT (id) DO UPDATE
            SET tx_hash = EXCLUDED.tx_hash,
                status = EXCLUDED.status,
                retry_count = EXCLUDED.retry_count,
                gas_used = EXCLUDED.gas_used,
                error = EXCLUDED.error,
                updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(tx.id)
    .bind(tx.user_id)
    .bind(&tx.tx_hash)
    .bind(serde_json::to_string(&tx.status)?)
    .bind(tx.retry_count as i32)
    .bind(tx.gas_used.map(|g| g as i64))
    .bind(&tx.error)
    .bind(tx.created_at)
    .bind(tx.updated_at)
    .execute(state.db())
    .await?;

    Ok(())
}

/// Retrieve transaction by ID.
pub async fn get_transaction(state: &AppState, tx_id: &Uuid) -> Result<Option<GaslessTransaction>> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, tx_hash, status, retry_count, gas_used, error,
               EXTRACT(EPOCH FROM created_at)::bigint as created_at,
               EXTRACT(EPOCH FROM updated_at)::bigint as updated_at
        FROM gasless_transactions
        WHERE id = $1
        "#,
    )
    .bind(tx_id)
    .fetch_optional(state.db())
    .await?;

    match row {
        None => Ok(None),
        Some(r) => {
            let status_str: String = r.get("status");
            let status: TxStatus = serde_json::from_str(&format!("\"{status_str}\""))?;

            Ok(Some(GaslessTransaction {
                id: r.get("id"),
                user_id: r.get("user_id"),
                tx_hash: r.get("tx_hash"),
                status,
                retry_count: r.get::<i32, _>("retry_count") as u32,
                gas_used: r.get::<Option<i64>, _>("gas_used").map(|g| g as u64),
                error: r.get("error"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }))
        }
    }
}
