//! Wallet balance and transaction tracking service.
//!
//! Handles:
//! - Atomic balance updates with transaction logging
//! - Transaction history retrieval
//! - Balance validation before operations
//! - Initial balance setup (1000 tokens)

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::state::AppState;

/// Initial balance for new users (1000 tokens).
pub const INITIAL_BALANCE: i64 = 1000;

/// Transaction types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    WagerStake,
    WagerWin,
    WagerRefund,
    ReferralBonus,
    InitialBalance,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "deposit",
            Self::Withdrawal => "withdrawal",
            Self::WagerStake => "wager_stake",
            Self::WagerWin => "wager_win",
            Self::WagerRefund => "wager_refund",
            Self::ReferralBonus => "referral_bonus",
            Self::InitialBalance => "initial_balance",
        }
    }
}

/// Wallet transaction record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: i64,
    pub transaction_type: TransactionType,
    pub balance_before: i64,
    pub balance_after: i64,
    pub reference_id: Option<Uuid>,
    pub description: Option<String>,
    pub created_at: i64,
}

// ---------------------------------------------------------------------------
// Balance operations
// ---------------------------------------------------------------------------

/// Get user's current balance.
pub async fn get_balance(state: &AppState, user_id: &Uuid) -> Result<i64> {
    let row = sqlx::query("SELECT balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(state.db())
        .await
        .context("Failed to fetch user balance")?;

    Ok(row.get("balance"))
}

/// Validate that user has sufficient balance for an operation.
pub async fn validate_balance(state: &AppState, user_id: &Uuid, required: i64) -> Result<()> {
    let balance = get_balance(state, user_id).await?;
    if balance < required {
        return Err(anyhow!(
            "Insufficient balance: have {balance}, need {required}"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Atomic balance updates
// ---------------------------------------------------------------------------

/// Update user balance atomically and log the transaction.
///
/// This function:
/// 1. Starts a database transaction
/// 2. Locks the user row (SELECT FOR UPDATE)
/// 3. Validates the balance change
/// 4. Updates the balance
/// 5. Logs the transaction
/// 6. Commits
pub async fn update_balance(
    state: &AppState,
    user_id: Uuid,
    amount: i64,
    tx_type: TransactionType,
    reference_id: Option<Uuid>,
    description: Option<String>,
) -> Result<WalletTransaction> {
    let mut tx = state.db().begin().await?;

    // Lock user row and get current balance
    let row = sqlx::query("SELECT balance FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to lock user row")?;

    let balance_before: i64 = row.get("balance");
    let balance_after = balance_before + amount;

    // Validate: balance cannot go negative
    if balance_after < 0 {
        return Err(anyhow!(
            "Balance would go negative: {balance_before} + {amount} = {balance_after}"
        ));
    }

    // Update balance
    sqlx::query("UPDATE users SET balance = $1, updated_at = NOW() WHERE id = $2")
        .bind(balance_after)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .context("Failed to update balance")?;

    // Log transaction
    let tx_id = Uuid::new_v4();
    let created_at = Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (id, user_id, amount, transaction_type, balance_before, balance_after, reference_id, description, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9))
        "#,
    )
    .bind(tx_id)
    .bind(user_id)
    .bind(amount)
    .bind(tx_type.as_str())
    .bind(balance_before)
    .bind(balance_after)
    .bind(reference_id)
    .bind(&description)
    .bind(created_at)
    .execute(&mut *tx)
    .await
    .context("Failed to log transaction")?;

    // Commit
    tx.commit().await?;

    Ok(WalletTransaction {
        id: tx_id,
        user_id,
        amount,
        transaction_type: tx_type,
        balance_before,
        balance_after,
        reference_id,
        description,
        created_at,
    })
}

/// Deduct balance (negative amount).
pub async fn deduct_balance(
    state: &AppState,
    user_id: Uuid,
    amount: i64,
    tx_type: TransactionType,
    reference_id: Option<Uuid>,
    description: Option<String>,
) -> Result<WalletTransaction> {
    if amount <= 0 {
        return Err(anyhow!("Deduct amount must be positive"));
    }
    update_balance(state, user_id, -amount, tx_type, reference_id, description).await
}

/// Add balance (positive amount).
pub async fn add_balance(
    state: &AppState,
    user_id: Uuid,
    amount: i64,
    tx_type: TransactionType,
    reference_id: Option<Uuid>,
    description: Option<String>,
) -> Result<WalletTransaction> {
    if amount <= 0 {
        return Err(anyhow!("Add amount must be positive"));
    }
    update_balance(state, user_id, amount, tx_type, reference_id, description).await
}

// ---------------------------------------------------------------------------
// Transaction history
// ---------------------------------------------------------------------------

/// Get user's transaction history with pagination.
pub async fn get_transaction_history(
    state: &AppState,
    user_id: &Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<WalletTransaction>> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, amount, transaction_type, balance_before, balance_after, reference_id, description,
               EXTRACT(EPOCH FROM created_at)::bigint as created_at
        FROM wallet_transactions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(state.db())
    .await?;

    let mut transactions = Vec::new();
    for row in rows {
        let tx_type_str: String = row.get("transaction_type");
        let tx_type = match tx_type_str.as_str() {
            "deposit" => TransactionType::Deposit,
            "withdrawal" => TransactionType::Withdrawal,
            "wager_stake" => TransactionType::WagerStake,
            "wager_win" => TransactionType::WagerWin,
            "wager_refund" => TransactionType::WagerRefund,
            "referral_bonus" => TransactionType::ReferralBonus,
            "initial_balance" => TransactionType::InitialBalance,
            _ => continue,
        };

        transactions.push(WalletTransaction {
            id: row.get("id"),
            user_id: row.get("user_id"),
            amount: row.get("amount"),
            transaction_type: tx_type,
            balance_before: row.get("balance_before"),
            balance_after: row.get("balance_after"),
            reference_id: row.get("reference_id"),
            description: row.get("description"),
            created_at: row.get("created_at"),
        });
    }

    Ok(transactions)
}

/// Get transaction count for a user.
pub async fn get_transaction_count(state: &AppState, user_id: &Uuid) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM wallet_transactions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(state.db())
        .await?;

    Ok(row.get("count"))
}

// ---------------------------------------------------------------------------
// Initial balance setup
// ---------------------------------------------------------------------------

/// Initialize wallet with starting balance (called during user registration).
pub async fn initialize_wallet(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<()> {
    let tx_id = Uuid::new_v4();
    let created_at = Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (id, user_id, amount, transaction_type, balance_before, balance_after, description, created_at)
        VALUES ($1, $2, $3, 'initial_balance', 0, $3, 'Welcome bonus', to_timestamp($4))
        "#,
    )
    .bind(tx_id)
    .bind(user_id)
    .bind(INITIAL_BALANCE)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
