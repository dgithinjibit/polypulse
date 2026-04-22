//! Paymaster API routes for gasless transactions.

use axum::{extract::{Path, State}, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    services::paymaster::{
        check_rate_limit, get_transaction, get_total_gas_expenditure,
        get_user_gas_expenditure, relay_transaction, GaslessTransaction,
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RelayTransactionRequest {
    /// Base64-encoded transaction data
    pub tx_data: String,
}

#[derive(Debug, Serialize)]
pub struct RelayTransactionResponse {
    pub transaction_id: Uuid,
    pub tx_hash: Option<String>,
    pub status: String,
    pub gas_used: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitResponse {
    pub allowed: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct GasExpenditureResponse {
    pub user_id: Uuid,
    pub total_gas_used: u64,
}

#[derive(Debug, Serialize)]
pub struct PlatformGasExpenditureResponse {
    pub total_gas_used: u64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/paymaster/relay
///
/// Submit a transaction for gasless execution. The paymaster will sponsor
/// the gas fees and retry up to 3 times on failure.
pub async fn relay(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Json(body): Json<RelayTransactionRequest>,
) -> AppResult<(StatusCode, Json<RelayTransactionResponse>)> {
    // Decode transaction data
    use base64::Engine as _;
    let tx_data = base64::engine::general_purpose::STANDARD
        .decode(&body.tx_data)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64 tx_data: {e}")))?;

    // Relay transaction
    let tx = relay_transaction(&state, claims.sub, &tx_data)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Transaction relay failed: {e}")))?;

    Ok((
        StatusCode::OK,
        Json(RelayTransactionResponse {
            transaction_id: tx.id,
            tx_hash: tx.tx_hash,
            status: format!("{:?}", tx.status).to_lowercase(),
            gas_used: tx.gas_used,
        }),
    ))
}

/// GET /api/v1/paymaster/rate-limit
///
/// Check if the authenticated user has exceeded the rate limit for gasless
/// transactions (10 per hour).
pub async fn rate_limit_check(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
) -> AppResult<Json<RateLimitResponse>> {
    let allowed = check_rate_limit(&state, &claims.sub)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Rate limit check failed: {e}")))?;

    let message = if allowed {
        "Rate limit OK".to_string()
    } else {
        "Rate limit exceeded (10 transactions per hour)".to_string()
    };

    Ok(Json(RateLimitResponse { allowed, message }))
}

/// GET /api/v1/paymaster/transactions/:id
///
/// Retrieve a gasless transaction by ID.
pub async fn get_tx(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(tx_id): Path<Uuid>,
) -> AppResult<Json<GaslessTransaction>> {
    let tx = get_transaction(&state, &tx_id)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch transaction: {e}")))?
        .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

    // Verify ownership
    if tx.user_id != claims.sub {
        return Err(AppError::Forbidden(
            "You do not have access to this transaction".to_string(),
        ));
    }

    Ok(Json(tx))
}

/// GET /api/v1/paymaster/gas-expenditure
///
/// Get the authenticated user's total gas expenditure.
pub async fn user_gas_expenditure(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
) -> AppResult<Json<GasExpenditureResponse>> {
    let total = get_user_gas_expenditure(&state, &claims.sub)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch gas expenditure: {e}")))?;

    Ok(Json(GasExpenditureResponse {
        user_id: claims.sub,
        total_gas_used: total,
    }))
}

/// GET /api/v1/paymaster/gas-expenditure/platform
///
/// Get the platform's total gas expenditure (admin only).
pub async fn platform_gas_expenditure(
    State(state): State<AppState>,
) -> AppResult<Json<PlatformGasExpenditureResponse>> {
    let total = get_total_gas_expenditure(&state)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to fetch gas expenditure: {e}")))?;

    Ok(Json(PlatformGasExpenditureResponse {
        total_gas_used: total,
    }))
}
