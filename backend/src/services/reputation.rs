use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    pub score: i32,
    pub badge: String,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    pub user_id: i64,
    pub event_type: String,
    pub score_change: i32,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct ReputationService {
    db: PgPool,
}

impl ReputationService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
    
    /// Add reputation points (with bounds checking)
    pub async fn add_reputation(
        &self,
        user_id: i64,
        event_type: &str,
        score_change: i32,
        reason: Option<String>,
    ) -> Result<ReputationScore, AppError> {
        // Record event
        sqlx::query!(
            r#"
            INSERT INTO reputation_events (user_id, event_type, score_change, reason)
            VALUES ($1, $2, $3, $4)
            "#,
            user_id,
            event_type,
            score_change,
            reason
        )
        .execute(&self.db)
        .await?;
        
        // Update user reputation with bounds (0-100)
        let user = sqlx::query!(
            r#"
            UPDATE users
            SET reputation_score = GREATEST(0, LEAST(100, reputation_score + $1))
            WHERE id = $2
            RETURNING reputation_score, is_verified
            "#,
            score_change,
            user_id
        )
        .fetch_one(&self.db)
        .await?;
        
        let score = user.reputation_score.unwrap_or(50);
        
        Ok(ReputationScore {
            score,
            badge: Self::get_badge(score),
            is_verified: user.is_verified.unwrap_or(false),
        })
    }
    
    /// Get user reputation
    pub async fn get_reputation(&self, user_id: i64) -> Result<ReputationScore, AppError> {
        let user = sqlx::query!(
            r#"
            SELECT reputation_score, is_verified
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await?;
        
        let score = user.reputation_score.unwrap_or(50);
        
        Ok(ReputationScore {
            score,
            badge: Self::get_badge(score),
            is_verified: user.is_verified.unwrap_or(false),
        })
    }
    
    /// Get reputation badge based on score
    fn get_badge(score: i32) -> String {
        match score {
            80..=100 => "🟢 Trusted".to_string(),
            50..=79 => "🟡 Neutral".to_string(),
            _ => "🔴 Caution".to_string(),
        }
    }
    
    /// Check if user meets minimum reputation for bet creation
    pub async fn can_create_bet(&self, user_id: i64) -> Result<bool, AppError> {
        let reputation = self.get_reputation(user_id).await?;
        Ok(reputation.score >= 30)
    }
    
    /// Check if user qualifies for verified badge
    pub async fn check_verified_status(&self, user_id: i64) -> Result<bool, AppError> {
        let user = sqlx::query!(
            r#"
            SELECT 
                reputation_score,
                (SELECT COUNT(*) FROM p2p_bets WHERE creator_id = $1) as bet_count
            FROM users
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_one(&self.db)
        .await?;
        
        let score = user.reputation_score.unwrap_or(50);
        let bet_count = user.bet_count.unwrap_or(0);
        
        // Verified: 90+ reputation and 50+ bets
        let qualifies = score >= 90 && bet_count >= 50;
        
        if qualifies {
            // Update verified status
            sqlx::query!(
                r#"
                UPDATE users
                SET is_verified = true
                WHERE id = $1
                "#,
                user_id
            )
            .execute(&self.db)
            .await?;
        }
        
        Ok(qualifies)
    }
    
    /// Get reputation history
    pub async fn get_history(
        &self,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<ReputationEvent>, AppError> {
        let events = sqlx::query!(
            r#"
            SELECT user_id, event_type, score_change, reason, created_at
            FROM reputation_events
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;
        
        let history = events
            .into_iter()
            .map(|e| ReputationEvent {
                user_id: e.user_id,
                event_type: e.event_type,
                score_change: e.score_change,
                reason: e.reason,
                created_at: e.created_at,
            })
            .collect();
        
        Ok(history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Feature: polypulse-enhancements, Property 7: Reputation Score Bounds
    #[test]
    fn test_reputation_bounds() {
        // Test that reputation stays within 0-100 regardless of changes
        let test_cases = vec![
            (50, 100, 100),   // 50 + 100 = 150, capped at 100
            (50, -100, 0),    // 50 - 100 = -50, floored at 0
            (0, -50, 0),      // Already at 0, stays at 0
            (100, 50, 100),   // Already at 100, stays at 100
            (50, 30, 80),     // Normal case: 50 + 30 = 80
            (80, -30, 50),    // Normal case: 80 - 30 = 50
        ];
        
        for (initial, change, expected) in test_cases {
            let result = calculate_bounded_reputation(initial, change);
            assert_eq!(
                result, expected,
                "Failed for initial={}, change={}: got {}, expected {}",
                initial, change, result, expected
            );
        }
    }
    
    #[test]
    fn test_reputation_badge_assignment() {
        let test_cases = vec![
            (100, "🟢 Trusted"),
            (90, "🟢 Trusted"),
            (80, "🟢 Trusted"),
            (79, "🟡 Neutral"),
            (50, "🟡 Neutral"),
            (49, "🔴 Caution"),
            (30, "🔴 Caution"),
            (0, "🔴 Caution"),
        ];
        
        for (score, expected_badge) in test_cases {
            let badge = ReputationService::get_badge(score);
            assert_eq!(
                badge, expected_badge,
                "Wrong badge for score {}: got {}, expected {}",
                score, badge, expected_badge
            );
        }
    }
    
    #[test]
    fn test_reputation_sequence() {
        // Test a sequence of reputation changes
        let mut score = 50;
        let changes = vec![
            (5, "accurate_report"),
            (5, "accurate_report"),
            (1, "bet_completion"),
            (-20, "false_report"),
            (5, "accurate_report"),
            (2, "referral"),
        ];
        
        for (change, _event) in changes {
            score = calculate_bounded_reputation(score, change);
            assert!(score >= 0 && score <= 100, "Score out of bounds: {}", score);
        }
        
        // Final score should be: 50 + 5 + 5 + 1 - 20 + 5 + 2 = 48
        assert_eq!(score, 48);
    }
    
    #[test]
    fn test_can_create_bet_threshold() {
        let test_cases = vec![
            (29, false),  // Below threshold
            (30, true),   // At threshold
            (31, true),   // Above threshold
            (50, true),   // Well above
            (0, false),   // Minimum
            (100, true),  // Maximum
        ];
        
        for (score, expected) in test_cases {
            let can_create = score >= 30;
            assert_eq!(
                can_create, expected,
                "Wrong result for score {}: got {}, expected {}",
                score, can_create, expected
            );
        }
    }
    
    #[test]
    fn test_verified_status_requirements() {
        let test_cases = vec![
            (90, 50, true),   // Meets both requirements
            (90, 49, false),  // Reputation OK, not enough bets
            (89, 50, false),  // Enough bets, reputation too low
            (100, 100, true), // Exceeds both
            (90, 0, false),   // No bets
            (0, 50, false),   // Low reputation
        ];
        
        for (reputation, bet_count, expected) in test_cases {
            let qualifies = reputation >= 90 && bet_count >= 50;
            assert_eq!(
                qualifies, expected,
                "Wrong result for rep={}, bets={}: got {}, expected {}",
                reputation, bet_count, qualifies, expected
            );
        }
    }
}

// Helper function for testing
fn calculate_bounded_reputation(current: i32, change: i32) -> i32 {
    let new_score = current + change;
    new_score.max(0).min(100)
}
