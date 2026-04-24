use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;

use crate::errors::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub activity_type: ActivityType,
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub bet_id: Option<String>,
    pub bet_question: Option<String>,
    pub amount: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    BetCreated,
    ParticipantJoined,
    OutcomeVerified,
    PayoutExecuted,
    AchievementUnlocked,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendingBet {
    pub bet_id: String,
    pub question: String,
    pub participant_count: i32,
    pub total_volume: i64,
    pub activity_count: usize,
    pub time_remaining: i64,
}

pub struct ActivityFeedService {
    db: PgPool,
}

impl ActivityFeedService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
    
    /// Record a new activity event
    pub async fn record_activity(&self, activity: Activity) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO activities 
            (id, activity_type, user_id, username, avatar_url, bet_id, bet_question, amount, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            activity.id,
            serde_json::to_string(&activity.activity_type)?,
            activity.user_id,
            activity.username,
            activity.avatar_url,
            activity.bet_id,
            activity.bet_question,
            activity.amount,
            activity.timestamp,
        )
        .execute(&self.db)
        .await?;
        
        Ok(())
    }
    
    /// Get recent activities
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<Activity>, AppError> {
        let records = sqlx::query!(
            r#"
            SELECT id, activity_type, user_id, username, avatar_url, 
                   bet_id, bet_question, amount, timestamp
            FROM activities
            ORDER BY timestamp DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await?;
        
        let activities = records
            .into_iter()
            .map(|r| Activity {
                id: r.id,
                activity_type: serde_json::from_str(&r.activity_type).unwrap_or(ActivityType::BetCreated),
                user_id: r.user_id,
                username: r.username,
                avatar_url: r.avatar_url,
                bet_id: r.bet_id,
                bet_question: r.bet_question,
                amount: r.amount,
                timestamp: r.timestamp,
            })
            .collect();
        
        Ok(activities)
    }
    
    /// Get trending bets (most active in last 24h)
    pub async fn get_trending_bets(&self, limit: i64) -> Result<Vec<TrendingBet>, AppError> {
        let cutoff = Utc::now() - Duration::hours(24);
        
        let records = sqlx::query!(
            r#"
            SELECT 
                bet_id,
                bet_question,
                COUNT(*) as activity_count,
                MAX(timestamp) as last_activity
            FROM activities
            WHERE bet_id IS NOT NULL
              AND timestamp > $1
            GROUP BY bet_id, bet_question
            ORDER BY activity_count DESC
            LIMIT $2
            "#,
            cutoff,
            limit
        )
        .fetch_all(&self.db)
        .await?;
        
        let mut trending_bets = Vec::new();
        
        for record in records {
            if let Some(bet_id) = record.bet_id {
                // Get bet details
                let bet_details = sqlx::query!(
                    r#"
                    SELECT 
                        question,
                        end_time,
                        (SELECT COUNT(*) FROM p2p_bet_participants WHERE bet_id = p2p_bets.id) as participant_count,
                        (SELECT COALESCE(SUM(stake_amount), 0) FROM p2p_bet_participants WHERE bet_id = p2p_bets.id) as total_volume
                    FROM p2p_bets
                    WHERE id = $1
                    "#,
                    bet_id.parse::<i64>().unwrap_or(0)
                )
                .fetch_optional(&self.db)
                .await?;
                
                if let Some(details) = bet_details {
                    let time_remaining = (details.end_time - Utc::now()).num_seconds();
                    
                    trending_bets.push(TrendingBet {
                        bet_id: bet_id.clone(),
                        question: record.bet_question.unwrap_or(details.question),
                        participant_count: details.participant_count.unwrap_or(0) as i32,
                        total_volume: details.total_volume.unwrap_or(0),
                        activity_count: record.activity_count.unwrap_or(0) as usize,
                        time_remaining,
                    });
                }
            }
        }
        
        Ok(trending_bets)
    }
    
    /// Get activity count for a specific bet
    pub async fn get_bet_activity_count(&self, bet_id: &str) -> Result<i64, AppError> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM activities
            WHERE bet_id = $1
            "#,
            bet_id
        )
        .fetch_one(&self.db)
        .await?;
        
        Ok(result.count.unwrap_or(0))
    }
    
    /// Get user activity history
    pub async fn get_user_activities(
        &self,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<Activity>, AppError> {
        let records = sqlx::query!(
            r#"
            SELECT id, activity_type, user_id, username, avatar_url,
                   bet_id, bet_question, amount, timestamp
            FROM activities
            WHERE user_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&self.db)
        .await?;
        
        let activities = records
            .into_iter()
            .map(|r| Activity {
                id: r.id,
                activity_type: serde_json::from_str(&r.activity_type).unwrap_or(ActivityType::BetCreated),
                user_id: r.user_id,
                username: r.username,
                avatar_url: r.avatar_url,
                bet_id: r.bet_id,
                bet_question: r.bet_question,
                amount: r.amount,
                timestamp: r.timestamp,
            })
            .collect();
        
        Ok(activities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    fn create_test_activity(activity_type: ActivityType, user_id: i64) -> Activity {
        Activity {
            id: Uuid::new_v4().to_string(),
            activity_type,
            user_id,
            username: format!("user{}", user_id),
            avatar_url: None,
            bet_id: Some("bet123".to_string()),
            bet_question: Some("Will it rain?".to_string()),
            amount: Some(1000000),
            timestamp: Utc::now(),
        }
    }
    
    #[test]
    fn test_activity_serialization() {
        let activity = create_test_activity(ActivityType::BetCreated, 1);
        let json = serde_json::to_string(&activity).unwrap();
        let deserialized: Activity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(activity.activity_type, deserialized.activity_type);
        assert_eq!(activity.user_id, deserialized.user_id);
    }
    
    #[test]
    fn test_activity_type_serialization() {
        let types = vec![
            ActivityType::BetCreated,
            ActivityType::ParticipantJoined,
            ActivityType::OutcomeVerified,
            ActivityType::PayoutExecuted,
            ActivityType::AchievementUnlocked,
        ];
        
        for activity_type in types {
            let json = serde_json::to_string(&activity_type).unwrap();
            let deserialized: ActivityType = serde_json::from_str(&json).unwrap();
            assert_eq!(activity_type, deserialized);
        }
    }
}
