use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    errors::{AppError, AppResult},
    lmsr,
    models::{Market, MarketPriceSnapshot, PollOption},
    state::AppState,
};

// ─── Response Types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketPrices {
    pub prices: HashMap<i64, f64>,
}

#[derive(Debug, Serialize)]
pub struct PriceSnapshot {
    pub yes_price: f64,
    pub no_price: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

/// GET /api/v1/markets/:poll_id/prices - Get current prices for all options
pub async fn get_market_prices(
    State(state): State<AppState>,
    Path(poll_id): Path<i64>,
) -> AppResult<Json<MarketPrices>> {
    let db = state.db();

    // Try to get from cache first
    if let Ok(Some(cached)) = crate::services::cache::get_cached_market_prices(&state, poll_id).await {
        if let Ok(market_prices) = serde_json::from_str::<MarketPrices>(&cached) {
            return Ok(Json(market_prices));
        }
    }

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

    // Parse shares
    let shares_map: HashMap<String, f64> =
        serde_json::from_value(market.shares_outstanding).unwrap_or_default();

    let shares: Vec<f64> = options
        .iter()
        .map(|opt| shares_map.get(&opt.id.to_string()).copied().unwrap_or(0.0))
        .collect();

    // Calculate prices
    let mut prices = HashMap::new();
    for (idx, opt) in options.iter().enumerate() {
        let price = lmsr::calculate_price(&shares, idx, market.liquidity_b);
        prices.insert(opt.id, price);
    }

    let market_prices = MarketPrices { prices };

    // Cache market prices (fallback on error)
    if let Ok(json) = serde_json::to_string(&market_prices) {
        let _ = crate::services::cache::cache_market_prices(&state, poll_id, &json).await;
    }

    Ok(Json(market_prices))
}

/// GET /api/v1/markets/:poll_id/history - Get price history
pub async fn get_price_history(
    State(state): State<AppState>,
    Path(poll_id): Path<i64>,
) -> AppResult<Json<Vec<PriceSnapshot>>> {
    let db = state.db();

    // Fetch market
    let market = sqlx::query_as::<_, Market>("SELECT * FROM markets WHERE poll_id = $1")
        .bind(poll_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

    // Fetch price snapshots
    let snapshots = sqlx::query_as::<_, MarketPriceSnapshot>(
        "SELECT * FROM market_price_snapshots WHERE market_id = $1 ORDER BY created_at DESC LIMIT 100"
    )
    .bind(market.id)
    .fetch_all(db)
    .await?;

    let result: Vec<PriceSnapshot> = snapshots
        .into_iter()
        .map(|snap| PriceSnapshot {
            yes_price: snap.yes_price,
            no_price: snap.no_price,
            created_at: snap.created_at,
        })
        .collect();

    Ok(Json(result))
}
