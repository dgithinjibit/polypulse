//! Wallet API routes.

use axum::{extract::{Query, State}, Extension, Json};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    // models::MpesaTransaction,
    services::{
        // mpesa::{normalise_phone, stk_push},
        wallet_transactions::{
            // add_balance, 
            get_balance, get_transaction_count, get_transaction_history,
            // TransactionType, 
            WalletTransaction,
        },
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TransactionHistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub transaction_type: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub balance: i64,
}

#[derive(Debug, Serialize)]
pub struct TransactionHistoryResponse {
    pub transactions: Vec<WalletTransaction>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ============================================================================
// M-PESA ENDPOINTS COMMENTED OUT - Using Freighter wallet instead
// ============================================================================
// All M-Pesa related code has been disabled. Use Freighter wallet for deposits.
// To re-enable, uncomment the sections below and in routes/mod.rs
// ============================================================================

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/wallet/balance
///
/// Get the authenticated user's current balance.
pub async fn get_user_balance(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
) -> AppResult<Json<BalanceResponse>> {
    let balance = get_balance(&state, &claims.sub)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch balance: {e}")))?;

    Ok(Json(BalanceResponse { balance }))
}

/// GET /api/v1/wallet/transactions
///
/// Get the authenticated user's transaction history with pagination.
pub async fn get_user_transactions(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Query(query): Query<TransactionHistoryQuery>,
) -> AppResult<Json<TransactionHistoryResponse>> {
    let transactions = get_transaction_history(&state, &claims.sub, query.limit, query.offset)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch transactions: {e}")))?;

    let total = get_transaction_count(&state, &claims.sub)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to count transactions: {e}")))?;

    Ok(Json(TransactionHistoryResponse {
        transactions,
        total,
        limit: query.limit,
        offset: query.offset,
    }))
}

// ============================================================================
// M-PESA ENDPOINTS COMMENTED OUT - Using Freighter wallet instead
// All M-Pesa deposit/callback/status functions have been removed.
// Use Freighter wallet for deposits instead.
// ============================================================================
