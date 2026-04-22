use axum::{
    extract::{Path, State},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{CommentLike, PollComment, User},
    state::AppState,
};

// ─── Request/Response Types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: i64,
    pub poll_id: i64,
    pub user_id: Uuid,
    pub username: String,
    pub content: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub like_count: i64,
    pub user_has_liked: bool,
}

#[derive(Debug, Serialize)]
pub struct CommentTree {
    pub id: i64,
    pub poll_id: i64,
    pub user_id: Uuid,
    pub username: String,
    pub content: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub like_count: i64,
    pub user_has_liked: bool,
    pub replies: Vec<CommentTree>,
}

#[derive(Debug, Serialize)]
pub struct LikeStatus {
    pub liked: bool,
    pub like_count: i64,
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

/// GET /api/v1/polls/:poll_id/comments - List comments with nested replies
pub async fn list_comments(
    State(state): State<AppState>,
    Path(poll_id): Path<i64>,
    auth_user: Option<Extension<AuthUser>>,
) -> AppResult<Json<Vec<CommentTree>>> {
    let db = state.db();
    let user_id = auth_user.as_ref().map(|Extension(AuthUser(claims))| claims.sub);

    // Query all top-level comments (parent_id IS NULL)
    let top_level_comments = sqlx::query_as::<_, PollComment>(
        r#"
        SELECT * FROM poll_comments
        WHERE poll_id = $1 AND parent_id IS NULL
        ORDER BY created_at DESC
        "#
    )
    .bind(poll_id)
    .fetch_all(db)
    .await?;

    // Build comment tree
    let mut result = Vec::new();
    for comment in top_level_comments {
        let tree = build_comment_tree(db, comment, user_id).await?;
        result.push(tree);
    }

    Ok(Json(result))
}

/// Recursively build comment tree with replies
async fn build_comment_tree(
    db: &sqlx::PgPool,
    comment: PollComment,
    user_id: Option<Uuid>,
) -> AppResult<CommentTree> {
    use futures::future::BoxFuture;
    
    fn build_tree_inner<'a>(
        db: &'a sqlx::PgPool,
        comment: PollComment,
        user_id: Option<Uuid>,
    ) -> BoxFuture<'a, AppResult<CommentTree>> {
        Box::pin(async move {
            // Get author details
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(comment.user_id)
                .fetch_optional(db)
                .await?
                .ok_or_else(|| AppError::NotFound("Comment author not found".to_string()))?;

            // Get like count
            let like_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM comment_likes WHERE comment_id = $1"
            )
            .bind(comment.id)
            .fetch_one(db)
            .await?;

            // Check if authenticated user has liked this comment
            let user_has_liked = if let Some(uid) = user_id {
                let count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM comment_likes WHERE comment_id = $1 AND user_id = $2"
                )
                .bind(comment.id)
                .bind(uid)
                .fetch_one(db)
                .await?;
                count > 0
            } else {
                false
            };

            // Recursively query replies
            let replies_data = sqlx::query_as::<_, PollComment>(
                r#"
                SELECT * FROM poll_comments
                WHERE parent_id = $1
                ORDER BY created_at ASC
                "#
            )
            .bind(comment.id)
            .fetch_all(db)
            .await?;

            let mut replies = Vec::new();
            for reply in replies_data {
                let reply_tree = build_tree_inner(db, reply, user_id).await?;
                replies.push(reply_tree);
            }

            Ok(CommentTree {
                id: comment.id,
                poll_id: comment.poll_id,
                user_id: comment.user_id,
                username: user.username,
                content: comment.content,
                parent_id: comment.parent_id,
                created_at: comment.created_at,
                like_count,
                user_has_liked,
                replies,
            })
        })
    }
    
    build_tree_inner(db, comment, user_id).await
}

/// POST /api/v1/polls/:poll_id/comments - Create a comment or reply
pub async fn create_comment(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(poll_id): Path<i64>,
    Json(req): Json<CreateCommentRequest>,
) -> AppResult<Json<CommentResponse>> {
    let db = state.db();
    let user_id = claims.sub;

    // Validate content is not empty
    if req.content.trim().is_empty() {
        return Err(AppError::BadRequest("Comment content cannot be empty".to_string()));
    }

    // Verify poll exists
    let poll_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM polls WHERE id = $1)")
        .bind(poll_id)
        .fetch_one(db)
        .await?;

    if !poll_exists {
        return Err(AppError::NotFound("Poll not found".to_string()));
    }

    // If parent_id is provided, verify it exists and belongs to the same poll
    if let Some(parent_id) = req.parent_id {
        let parent_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM poll_comments WHERE id = $1 AND poll_id = $2)"
        )
        .bind(parent_id)
        .bind(poll_id)
        .fetch_one(db)
        .await?;

        if !parent_exists {
            return Err(AppError::BadRequest("Parent comment not found or does not belong to this poll".to_string()));
        }
    }

    // Start transaction
    let mut tx = db.begin().await?;

    // Insert comment
    let comment = sqlx::query_as::<_, PollComment>(
        r#"
        INSERT INTO poll_comments (poll_id, user_id, content, parent_id)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#
    )
    .bind(poll_id)
    .bind(user_id)
    .bind(&req.content)
    .bind(req.parent_id)
    .fetch_one(&mut *tx)
    .await?;

    // Parse content for @mentions
    let mentioned_usernames = extract_mentions(&req.content);

    // For each mentioned username, query user and create notification
    for username in mentioned_usernames {
        // Query user by username
        let mentioned_user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1"
        )
        .bind(&username)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(mentioned_user) = mentioned_user {
            // Don't notify if user mentions themselves
            if mentioned_user.id != user_id {
                // Get the mentioning user's username
                let mentioning_user = sqlx::query_as::<_, User>(
                    "SELECT * FROM users WHERE id = $1"
                )
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;

                // Create notification
                sqlx::query(
                    r#"
                    INSERT INTO notifications (user_id, actor_id, notification_type, message, is_read)
                    VALUES ($1, $2, 'mention', $3, false)
                    "#
                )
                .bind(mentioned_user.id)
                .bind(user_id)
                .bind(format!("{} mentioned you in a comment", mentioning_user.username))
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    // Commit transaction
    tx.commit().await?;

    // Get author details for response
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await?;

    // Broadcast comment added event via WebSocket
    crate::ws::broadcast_comment_added(
        &state,
        poll_id,
        comment.id,
        user_id,
        req.content.clone(),
    ).await;

    // Return comment details
    Ok(Json(CommentResponse {
        id: comment.id,
        poll_id: comment.poll_id,
        user_id: comment.user_id,
        username: user.username,
        content: comment.content,
        parent_id: comment.parent_id,
        created_at: comment.created_at,
        like_count: 0,
        user_has_liked: false,
    }))
}

/// POST /api/v1/comments/:id/like - Toggle like on a comment
pub async fn toggle_like(
    State(state): State<AppState>,
    Extension(AuthUser(claims)): Extension<AuthUser>,
    Path(comment_id): Path<i64>,
) -> AppResult<Json<LikeStatus>> {
    let db = state.db();
    let user_id = claims.sub;

    // Verify comment exists
    let comment_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM poll_comments WHERE id = $1)"
    )
    .bind(comment_id)
    .fetch_one(db)
    .await?;

    if !comment_exists {
        return Err(AppError::NotFound("Comment not found".to_string()));
    }

    // Start transaction
    let mut tx = db.begin().await?;

    // Query if like exists for (comment_id, user_id)
    let existing_like = sqlx::query_as::<_, CommentLike>(
        "SELECT * FROM comment_likes WHERE comment_id = $1 AND user_id = $2"
    )
    .bind(comment_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let liked = if let Some(like) = existing_like {
        // Like exists, delete it (unlike)
        sqlx::query("DELETE FROM comment_likes WHERE id = $1")
            .bind(like.id)
            .execute(&mut *tx)
            .await?;
        false
    } else {
        // Like doesn't exist, insert it (like)
        sqlx::query(
            r#"
            INSERT INTO comment_likes (comment_id, user_id)
            VALUES ($1, $2)
            "#
        )
        .bind(comment_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        true
    };

    // Get updated like count
    let like_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM comment_likes WHERE comment_id = $1"
    )
    .bind(comment_id)
    .fetch_one(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    Ok(Json(LikeStatus { liked, like_count }))
}

// ─── Helper Functions ────────────────────────────────────────────────────────

/// Extract @mentions from text using regex
fn extract_mentions(text: &str) -> Vec<String> {
    let re = Regex::new(r"@([\w.@+-]+)").unwrap();
    re.captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}
