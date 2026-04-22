use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── Users ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub phone: Option<String>,
    pub balance: f64,
    pub is_staff: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// ─── Profiles ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Profile {
    pub id: i64,
    pub user_id: Uuid,
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub email_verification_token: Option<Uuid>,
    pub current_streak: i32,
    pub best_streak: i32,
    pub total_predictions: i32,
    pub correct_predictions: i32,
    pub polls_created_today: i32,
    pub last_poll_created_date: Option<NaiveDate>,
    pub referral_code: Option<String>,
}

// ─── Auth Nonces ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthNonce {
    pub id: i64,
    pub account_id: String,
    pub nonce: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

// ─── Omnichain Nonces ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OmnichainNonce {
    pub id: i64,
    pub chain: String,
    pub address: String,
    pub nonce: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

// ─── NEAR Accounts ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NearAccount {
    pub id: i64,
    pub user_id: Uuid,
    pub account_id: String,
    pub public_key: String,
    pub created_at: DateTime<Utc>,
}

// ─── Omnichain Accounts ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OmnichainAccount {
    pub id: i64,
    pub user_id: Uuid,
    pub chain: String,
    pub address: String,
    pub public_key: String,
    pub created_at: DateTime<Utc>,
}

// ─── User Sessions ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_fingerprint: Option<String>,
    pub ip_address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// ─── Poll Categories ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PollCategory {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

// ─── Polls ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Poll {
    pub id: i64,
    pub creator_id: Uuid,
    pub title: String,
    pub description: String,
    pub category_id: Option<i64>,
    pub status: String,
    pub is_free: bool,
    pub closes_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub winning_option_id: Option<i64>,
    pub resolution_criteria: String,
}

// ─── Poll Options ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PollOption {
    pub id: i64,
    pub poll_id: i64,
    pub text: String,
    pub is_yes: bool,
    pub order: i16,
}

// ─── Markets ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Market {
    pub id: i64,
    pub poll_id: i64,
    pub liquidity_b: f64,
    pub shares_outstanding: serde_json::Value,
}

// ─── Market Price Snapshots ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketPriceSnapshot {
    pub id: i64,
    pub market_id: i64,
    pub yes_price: f64,
    pub no_price: f64,
    pub created_at: DateTime<Utc>,
}

// ─── Bets ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bet {
    pub id: i64,
    pub user_id: Uuid,
    pub poll_id: i64,
    pub option_id: i64,
    pub amount: f64,
    pub shares: f64,
    pub created_at: DateTime<Utc>,
}

// ─── Market Positions ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketPosition {
    pub id: i64,
    pub user_id: Uuid,
    pub market_id: i64,
    pub option_shares: serde_json::Value,
    pub option_spent: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ─── Poll Comments ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PollComment {
    pub id: i64,
    pub poll_id: i64,
    pub user_id: Uuid,
    pub content: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// ─── Comment Likes ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommentLike {
    pub id: i64,
    pub comment_id: i64,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// ─── Notifications ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Notification {
    pub id: i64,
    pub user_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub notification_type: String,
    pub message: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

// ─── Challenges ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Challenge {
    pub id: i64,
    pub creator_id: Uuid,
    pub opponent_id: Option<Uuid>,
    pub question: String,
    pub amount: sqlx::types::BigDecimal,
    pub creator_choice: String,
    pub status: String,
    pub is_open: bool,
    pub poll_id: Option<i64>,
    pub expires_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub winner_id: Option<Uuid>,
    pub resolution_criteria: String,
    pub created_at: DateTime<Utc>,
}

// ─── Wallet Transactions ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WalletTransaction {
    pub id: i64,
    pub user_id: Uuid,
    pub amount: f64,
    pub transaction_type: String,
    pub balance_after: f64,
    pub description: String,
    pub related_poll_id: Option<i64>,
    pub related_bet_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// ─── M-Pesa Transactions ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MpesaTransaction {
    pub id: i64,
    pub user_id: Uuid,
    pub transaction_type: String,
    pub phone: String,
    pub amount: i32,
    pub checkout_request_id: String,
    pub merchant_request_id: String,
    pub mpesa_receipt: String,
    pub status: String,
    pub result_desc: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
