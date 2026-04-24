use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{errors::AppError, state::AppState, ws};

/// Notification types for P2P betting events
pub enum P2PNotificationType {
    BetEndingSoon,
    BetEnded,
    ParticipantJoined,
    OutcomeReported,
    OutcomeVerified,
    PayoutExecuted,
    BetDisputed,
}

impl P2PNotificationType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::BetEndingSoon => "bet_ending_soon",
            Self::BetEnded => "bet_ended",
            Self::ParticipantJoined => "participant_joined",
            Self::OutcomeReported => "outcome_reported",
            Self::OutcomeVerified => "outcome_verified",
            Self::PayoutExecuted => "payout_executed",
            Self::BetDisputed => "bet_disputed",
        }
    }
}

/// Create a notification for a user
pub async fn create_notification(
    db: &PgPool,
    user_id: Uuid,
    notification_type: P2PNotificationType,
    message: String,
    actor_id: Option<Uuid>,
    bet_id: Option<i64>,
) -> Result<i64, AppError> {
    let notification = sqlx::query!(
        r#"
        INSERT INTO notifications (user_id, notification_type, message, actor_id, bet_id, is_read)
        VALUES ($1, $2, $3, $4, $5, false)
        RETURNING id
        "#,
        user_id,
        notification_type.as_str(),
        message,
        actor_id,
        bet_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to create notification: {}", e)))?;

    Ok(notification.id)
}

/// Notify all participants of a bet
pub async fn notify_bet_participants(
    state: &AppState,
    bet_id: i64,
    notification_type: P2PNotificationType,
    message: String,
    actor_id: Option<Uuid>,
) -> Result<(), AppError> {
    // Get all participants including creator
    let participants = sqlx::query!(
        r#"
        SELECT DISTINCT u.id as user_id
        FROM users u
        WHERE u.id IN (
            SELECT u2.id FROM p2p_bets b
            JOIN users u2 ON b.creator_id::text = u2.id::text
            WHERE b.id = $1
            UNION
            SELECT u3.id FROM p2p_bet_participants p
            JOIN users u3 ON p.user_id::text = u3.id::text
            WHERE p.bet_id = $1
        )
        "#,
        bet_id
    )
    .fetch_all(state.db())
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to fetch participants: {}", e)))?;

    // Create notifications for each participant
    for participant in participants {
        let notification_id = create_notification(
            state.db(),
            participant.user_id,
            notification_type,
            message.clone(),
            actor_id,
            Some(bet_id),
        )
        .await?;

        // Send real-time notification via WebSocket
        ws::send_notification_to_user(
            state,
            participant.user_id,
            notification_id,
            notification_type.as_str().to_string(),
            message.clone(),
            actor_id,
        )
        .await;
    }

    Ok(())
}

/// Notify bet creator
pub async fn notify_bet_creator(
    state: &AppState,
    bet_id: i64,
    notification_type: P2PNotificationType,
    message: String,
    actor_id: Option<Uuid>,
) -> Result<(), AppError> {
    // Get bet creator
    let bet = sqlx::query!(
        "SELECT creator_id FROM p2p_bets WHERE id = $1",
        bet_id
    )
    .fetch_optional(state.db())
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to fetch bet: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Bet not found".to_string()))?;

    // Get creator UUID from users table
    let creator = sqlx::query!(
        "SELECT id FROM users WHERE id::text = $1::text",
        bet.creator_id.to_string()
    )
    .fetch_optional(state.db())
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to fetch creator: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Creator not found".to_string()))?;

    let notification_id = create_notification(
        state.db(),
        creator.id,
        notification_type,
        message.clone(),
        actor_id,
        Some(bet_id),
    )
    .await?;

    // Send real-time notification via WebSocket
    ws::send_notification_to_user(
        state,
        creator.id,
        notification_id,
        notification_type.as_str().to_string(),
        message,
        actor_id,
    )
    .await;

    Ok(())
}

/// Check for bets ending soon and send notifications
pub async fn check_bets_ending_soon(state: &AppState) -> Result<(), AppError> {
    let one_hour_from_now = Utc::now() + Duration::hours(1);
    let now = Utc::now();

    // Find bets ending in the next hour that haven't been notified
    let bets = sqlx::query!(
        r#"
        SELECT id, question, end_time
        FROM p2p_bets
        WHERE state IN ('Created', 'Active')
        AND end_time > $1
        AND end_time <= $2
        AND NOT EXISTS (
            SELECT 1 FROM notifications
            WHERE bet_id = p2p_bets.id
            AND notification_type = 'bet_ending_soon'
        )
        "#,
        now,
        one_hour_from_now
    )
    .fetch_all(state.db())
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to fetch ending bets: {}", e)))?;

    for bet in bets {
        let message = format!("Bet ending soon: \"{}\"", bet.question);
        notify_bet_participants(
            state,
            bet.id,
            P2PNotificationType::BetEndingSoon,
            message,
            None,
        )
        .await?;
    }

    Ok(())
}

/// Check for ended bets and send notifications
pub async fn check_ended_bets(state: &AppState) -> Result<(), AppError> {
    let now = Utc::now();

    // Find bets that just ended
    let bets = sqlx::query!(
        r#"
        SELECT id, question
        FROM p2p_bets
        WHERE state IN ('Created', 'Active')
        AND end_time <= $1
        AND NOT EXISTS (
            SELECT 1 FROM notifications
            WHERE bet_id = p2p_bets.id
            AND notification_type = 'bet_ended'
        )
        "#,
        now
    )
    .fetch_all(state.db())
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to fetch ended bets: {}", e)))?;

    for bet in bets {
        // Update bet state to Ended
        sqlx::query!(
            "UPDATE p2p_bets SET state = 'Ended' WHERE id = $1",
            bet.id
        )
        .execute(state.db())
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update bet state: {}", e)))?;

        let message = format!("Bet has ended: \"{}\". Please report the outcome.", bet.question);
        notify_bet_participants(
            state,
            bet.id,
            P2PNotificationType::BetEnded,
            message,
            None,
        )
        .await?;
    }

    Ok(())
}

/// Clean up old notifications (30-day retention)
pub async fn cleanup_old_notifications(db: &PgPool) -> Result<u64, AppError> {
    let thirty_days_ago = Utc::now() - Duration::days(30);

    let result = sqlx::query!(
        "DELETE FROM notifications WHERE created_at < $1",
        thirty_days_ago
    )
    .execute(db)
    .await
    .map_err(|e| AppError::DatabaseError(format!("Failed to cleanup notifications: {}", e)))?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_strings() {
        assert_eq!(P2PNotificationType::BetEndingSoon.as_str(), "bet_ending_soon");
        assert_eq!(P2PNotificationType::BetEnded.as_str(), "bet_ended");
        assert_eq!(P2PNotificationType::ParticipantJoined.as_str(), "participant_joined");
        assert_eq!(P2PNotificationType::OutcomeReported.as_str(), "outcome_reported");
        assert_eq!(P2PNotificationType::OutcomeVerified.as_str(), "outcome_verified");
        assert_eq!(P2PNotificationType::PayoutExecuted.as_str(), "payout_executed");
        assert_eq!(P2PNotificationType::BetDisputed.as_str(), "bet_disputed");
    }
}
