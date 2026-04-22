//! Categories API routes.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::{
    errors::AppResult,
    state::AppState,
};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/categories
///
/// Get all poll categories.
pub async fn list_categories(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<Category>>> {
    let rows = sqlx::query_as!(
        Category,
        r#"
        SELECT id, name, slug
        FROM poll_categories
        ORDER BY name ASC
        "#
    )
    .fetch_all(state.db())
    .await?;

    Ok(Json(rows))
}
