use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub user_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub score: i64,
    pub total_bets: i64,
    pub level: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLevel {
    pub level: i32,
    pub xp: i32,
    pub next_level_xp: i32,
    pub level_name: String,
}

pub struct LeaderboardService {
    db: PgPool,
}

impl LeaderboardService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get top earners leaderboard
    pub async fn get_top_earners(&self, limit: i32) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        let entries = sqlx::query_as!(
            LeaderboardEntry,
            r#"
            SELECT 
                ROW_NUMBER() OVER (ORDER BY COALESCE(SUM(pp.stake), 0) DESC) as "rank!",
                u.id as "user_id!",
                COALESCE(u.username, 'Anonymous') as "username!",
                u.avatar_url,
                COALESCE(SUM(pp.stake), 0) as "score!",
                COUNT(DISTINCT pb.id) as "total_bets!",
                COALESCE(u.level, 1) as "level!"
            FROM users u
            LEFT JOIN p2p_bet_participants pp ON pp.user_id = u.id
            LEFT JOIN p2p_bets pb ON pb.id = pp.bet_id
            GROUP BY u.id
            ORDER BY score DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(entries)
    }

    /// Get best predictors leaderboard (by win rate)
    pub async fn get_best_predictors(&self, limit: i32) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        let entries = sqlx::query_as!(
            LeaderboardEntry,
            r#"
            SELECT 
                ROW_NUMBER() OVER (ORDER BY COALESCE(u.xp, 0) DESC) as "rank!",
                u.id as "user_id!",
                COALESCE(u.username, 'Anonymous') as "username!",
                u.avatar_url,
                COALESCE(u.xp, 0)::bigint as "score!",
                COUNT(DISTINCT pp.bet_id) as "total_bets!",
                COALESCE(u.level, 1) as "level!"
            FROM users u
            LEFT JOIN p2p_bet_participants pp ON pp.user_id = u.id
            GROUP BY u.id
            HAVING COUNT(DISTINCT pp.bet_id) >= 5
            ORDER BY score DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(entries)
    }

    /// Get most active users leaderboard
    pub async fn get_most_active(&self, limit: i32) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        let entries = sqlx::query_as!(
            LeaderboardEntry,
            r#"
            SELECT 
                ROW_NUMBER() OVER (ORDER BY COUNT(DISTINCT pp.bet_id) DESC) as "rank!",
                u.id as "user_id!",
                COALESCE(u.username, 'Anonymous') as "username!",
                u.avatar_url,
                COUNT(DISTINCT pp.bet_id) as "score!",
                COUNT(DISTINCT pp.bet_id) as "total_bets!",
                COALESCE(u.level, 1) as "level!"
            FROM users u
            LEFT JOIN p2p_bet_participants pp ON pp.user_id = u.id
            GROUP BY u.id
            ORDER BY score DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db)
        .await?;

        Ok(entries)
    }

    /// Add XP to user
    pub async fn add_xp(&self, user_id: i64, xp_amount: i32, reason: &str) -> Result<UserLevel, sqlx::Error> {
        // Update XP
        let user = sqlx::query!(
            r#"
            UPDATE users
            SET xp = COALESCE(xp, 0) + $1,
                updated_at = NOW()
            WHERE id = $2
            RETURNING xp
            "#,
            xp_amount,
            user_id
        )
        .fetch_one(&self.db)
        .await?;

        let xp = user.xp.unwrap_or(0);
        let level = Self::calculate_level(xp);

        // Update level if changed
        sqlx::query!(
            "UPDATE users SET level = $1 WHERE id = $2",
            level,
            user_id
        )
        .execute(&self.db)
        .await?;

        Ok(UserLevel {
            level,
            xp,
            next_level_xp: Self::xp_for_level(level + 1),
            level_name: Self::level_name(level),
        })
    }

    /// Calculate level from XP
    fn calculate_level(xp: i32) -> i32 {
        match xp {
            0..=99 => 1,
            100..=499 => 2,
            500..=1999 => 3,
            _ => 4,
        }
    }

    /// Get XP required for level
    fn xp_for_level(level: i32) -> i32 {
        match level {
            1 => 0,
            2 => 100,
            3 => 500,
            4 => 2000,
            _ => 2000,
        }
    }

    /// Get level name
    fn level_name(level: i32) -> String {
        match level {
            1 => "Bronze".to_string(),
            2 => "Silver".to_string(),
            3 => "Gold".to_string(),
            4 => "Diamond".to_string(),
            _ => "Diamond".to_string(),
        }
    }

    /// Update win streak
    pub async fn update_win_streak(&self, user_id: i64, won: bool) -> Result<i32, sqlx::Error> {
        if won {
            let result = sqlx::query!(
                r#"
                UPDATE users
                SET win_streak = COALESCE(win_streak, 0) + 1
                WHERE id = $1
                RETURNING win_streak
                "#,
                user_id
            )
            .fetch_one(&self.db)
            .await?;

            Ok(result.win_streak.unwrap_or(0))
        } else {
            sqlx::query!(
                "UPDATE users SET win_streak = 0 WHERE id = $1",
                user_id
            )
            .execute(&self.db)
            .await?;

            Ok(0)
        }
    }

    /// Award achievement
    pub async fn award_achievement(&self, user_id: i64, achievement_code: &str) -> Result<bool, sqlx::Error> {
        // Check if already has achievement
        let existing = sqlx::query!(
            r#"
            SELECT id FROM user_achievements ua
            JOIN achievements a ON a.id = ua.achievement_id
            WHERE ua.user_id = $1 AND a.code = $2
            "#,
            user_id,
            achievement_code
        )
        .fetch_optional(&self.db)
        .await?;

        if existing.is_some() {
            return Ok(false); // Already has achievement
        }

        // Get achievement
        let achievement = sqlx::query!(
            "SELECT id, xp_reward FROM achievements WHERE code = $1",
            achievement_code
        )
        .fetch_one(&self.db)
        .await?;

        // Award achievement
        sqlx::query!(
            "INSERT INTO user_achievements (user_id, achievement_id) VALUES ($1, $2)",
            user_id,
            achievement.id
        )
        .execute(&self.db)
        .await?;

        // Award XP
        if let Some(xp_reward) = achievement.xp_reward {
            self.add_xp(user_id, xp_reward, &format!("Achievement: {}", achievement_code)).await?;
        }

        Ok(true)
    }
}
