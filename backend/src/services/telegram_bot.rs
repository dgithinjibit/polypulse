use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub from: TelegramUser,
    pub chat: TelegramChat,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: TelegramUser,
    pub message: Option<TelegramMessage>,
    pub data: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageRequest {
    pub chat_id: i64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<InlineKeyboardMarkup>,
}

#[derive(Debug, Serialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Serialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data: Option<String>,
}

pub struct TelegramBot {
    token: String,
    db: PgPool,
    http_client: reqwest::Client,
}

impl TelegramBot {
    pub fn new(token: String, db: PgPool) -> Self {
        Self {
            token,
            db,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn handle_webhook(&self, update: TelegramUpdate) -> Result<(), BotError> {
        if let Some(msg) = update.message {
            self.handle_message(msg).await?;
        } else if let Some(callback) = update.callback_query {
            self.handle_callback(callback).await?;
        }
        Ok(())
    }

    async fn handle_message(&self, msg: TelegramMessage) -> Result<(), BotError> {
        let text = msg.text.ok_or(BotError::NoText)?;
        let parts: Vec<&str> = text.split_whitespace().collect();

        match parts.first() {
            Some(&"/start") => self.cmd_start(msg.chat.id, msg.from).await,
            Some(&"/bet") => self.cmd_create_bet(msg).await,
            Some(&"/mybets") => self.cmd_my_bets(msg.from.id, msg.chat.id).await,
            Some(&"/positions") => self.cmd_positions(msg.from.id, msg.chat.id).await,
            Some(&"/leaderboard") => self.cmd_leaderboard(msg.chat.id).await,
            Some(&"/help") => self.cmd_help(msg.chat.id).await,
            _ => self.send_message(msg.chat.id, "Unknown command. Use /help for assistance.").await,
        }
    }

    async fn cmd_start(&self, chat_id: i64, user: TelegramUser) -> Result<(), BotError> {
        // Register telegram user
        sqlx::query!(
            r#"
            INSERT INTO telegram_users (telegram_id, telegram_username)
            VALUES ($1, $2)
            ON CONFLICT (telegram_id) DO UPDATE
            SET telegram_username = $2
            "#,
            user.id,
            user.username
        )
        .execute(&self.db)
        .await?;

        let message = format!(
            "🎲 *Welcome to PolyPulse!*\n\n\
             Create and join peer-to-peer bets on anything.\n\n\
             *Commands:*\n\
             /bet [question] [amount] - Create a bet\n\
             /mybets - View your created bets\n\
             /positions - View your positions\n\
             /leaderboard - View top users\n\
             /help - Show this message\n\n\
             Example: `/bet Will it rain tomorrow? 10`"
        );

        self.send_message_markdown(chat_id, &message).await
    }

    async fn cmd_create_bet(&self, msg: TelegramMessage) -> Result<(), BotError> {
        let text = msg.text.unwrap();
        let parts: Vec<&str> = text.split_whitespace().collect();

        if parts.len() < 3 {
            return self.send_message(
                msg.chat.id,
                "Usage: /bet [question] [amount]\nExample: /bet Will it rain? 10"
            ).await;
        }

        // Parse amount (last part)
        let amount_str = parts.last().unwrap();
        let amount = amount_str.parse::<f64>()
            .map_err(|_| BotError::InvalidAmount)?;

        if amount <= 0.0 {
            return self.send_message(msg.chat.id, "Amount must be positive").await;
        }

        // Parse question (everything between /bet and amount)
        let question = parts[1..parts.len()-1].join(" ");

        if question.len() < 10 {
            return self.send_message(msg.chat.id, "Question must be at least 10 characters").await;
        }

        if question.len() > 200 {
            return self.send_message(msg.chat.id, "Question must be at most 200 characters").await;
        }

        if !question.ends_with('?') {
            return self.send_message(msg.chat.id, "Question must end with a question mark").await;
        }

        // Create bet in database
        let stake_stroops = (amount * 10_000_000.0) as i64;
        let end_time = chrono::Utc::now() + chrono::Duration::hours(24);

        let bet = sqlx::query!(
            r#"
            INSERT INTO p2p_bets (
                creator_id, question, question_normalized, question_slug,
                stake_amount, end_time, state, shareable_url_hash, is_multi_participant
            )
            VALUES (
                (SELECT user_id FROM telegram_users WHERE telegram_id = $1),
                $2, $3, $4, $5, $6, 'Created', $7, true
            )
            RETURNING id
            "#,
            msg.from.id,
            question,
            question.to_lowercase(),
            Self::create_slug(&question),
            stake_stroops,
            end_time,
            format!("tg_bet_{}", uuid::Uuid::new_v4())
        )
        .fetch_one(&self.db)
        .await?;

        // Generate shareable link
        let share_url = format!("https://t.me/polypulse_bot?start=bet_{}", bet.id);

        // Send response with inline keyboard
        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: vec![
                vec![
                    InlineKeyboardButton {
                        text: "Join Bet".to_string(),
                        url: Some(share_url.clone()),
                        callback_data: None,
                    },
                    InlineKeyboardButton {
                        text: "Share".to_string(),
                        url: None,
                        callback_data: Some(format!("share_{}", bet.id)),
                    },
                ],
            ],
        };

        let message = format!(
            "🎲 *Bet Created!*\n\n\
             Question: {}\n\
             Stake: {} XLM\n\
             Ends: {}\n\n\
             Share this bet with friends!",
            question,
            amount,
            end_time.format("%Y-%m-%d %H:%M UTC")
        );

        self.send_message_with_keyboard(msg.chat.id, &message, keyboard).await
    }

    async fn cmd_my_bets(&self, telegram_id: i64, chat_id: i64) -> Result<(), BotError> {
        let bets = sqlx::query!(
            r#"
            SELECT pb.id, pb.question, pb.stake_amount, pb.state, pb.participant_count
            FROM p2p_bets pb
            JOIN telegram_users tu ON tu.user_id = pb.creator_id
            WHERE tu.telegram_id = $1
            ORDER BY pb.created_at DESC
            LIMIT 10
            "#,
            telegram_id
        )
        .fetch_all(&self.db)
        .await?;

        if bets.is_empty() {
            return self.send_message(chat_id, "You haven't created any bets yet.").await;
        }

        let mut message = "*Your Bets:*\n\n".to_string();
        for bet in bets {
            message.push_str(&format!(
                "• {} ({})\n  Stake: {} XLM | Participants: {}\n\n",
                bet.question,
                bet.state,
                bet.stake_amount as f64 / 10_000_000.0,
                bet.participant_count.unwrap_or(0)
            ));
        }

        self.send_message_markdown(chat_id, &message).await
    }

    async fn cmd_positions(&self, telegram_id: i64, chat_id: i64) -> Result<(), BotError> {
        let positions = sqlx::query!(
            r#"
            SELECT pb.question, pp.position, pp.stake, pb.state
            FROM p2p_bet_participants pp
            JOIN p2p_bets pb ON pb.id = pp.bet_id
            JOIN telegram_users tu ON tu.user_id = pp.user_id
            WHERE tu.telegram_id = $1
            ORDER BY pp.joined_at DESC
            LIMIT 10
            "#,
            telegram_id
        )
        .fetch_all(&self.db)
        .await?;

        if positions.is_empty() {
            return self.send_message(chat_id, "You don't have any active positions.").await;
        }

        let mut message = "*Your Positions:*\n\n".to_string();
        for pos in positions {
            let position_str = if pos.position { "Yes" } else { "No" };
            message.push_str(&format!(
                "• {} ({})\n  Position: {} | Stake: {} XLM\n\n",
                pos.question,
                pos.state,
                position_str,
                pos.stake as f64 / 10_000_000.0
            ));
        }

        self.send_message_markdown(chat_id, &message).await
    }

    async fn cmd_leaderboard(&self, chat_id: i64) -> Result<(), BotError> {
        let leaders = sqlx::query!(
            r#"
            SELECT u.username, u.xp, u.level
            FROM users u
            ORDER BY u.xp DESC
            LIMIT 10
            "#
        )
        .fetch_all(&self.db)
        .await?;

        let mut message = "🏆 *Leaderboard*\n\n".to_string();
        for (i, leader) in leaders.iter().enumerate() {
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            message.push_str(&format!(
                "{} {}. {} - {} XP (Level {})\n",
                medal,
                i + 1,
                leader.username.as_ref().unwrap_or(&"Anonymous".to_string()),
                leader.xp.unwrap_or(0),
                leader.level.unwrap_or(1)
            ));
        }

        self.send_message_markdown(chat_id, &message).await
    }

    async fn cmd_help(&self, chat_id: i64) -> Result<(), BotError> {
        let message = "*PolyPulse Bot Commands:*\n\n\
             /bet [question] [amount] - Create a bet\n\
             /mybets - View your created bets\n\
             /positions - View your positions\n\
             /leaderboard - View top users\n\
             /help - Show this message\n\n\
             *Example:*\n\
             `/bet Will Bitcoin hit $100k by end of year? 50`";

        self.send_message_markdown(chat_id, &message).await
    }

    async fn handle_callback(&self, callback: CallbackQuery) -> Result<(), BotError> {
        if let Some(data) = callback.data {
            if data.starts_with("share_") {
                let bet_id = data.strip_prefix("share_").unwrap();
                let share_url = format!("https://t.me/polypulse_bot?start=bet_{}", bet_id);
                
                // Answer callback query
                self.answer_callback_query(&callback.id, Some("Share this link with friends!")).await?;
            }
        }
        Ok(())
    }

    async fn send_message(&self, chat_id: i64, text: &str) -> Result<(), BotError> {
        let request = SendMessageRequest {
            chat_id,
            text: text.to_string(),
            parse_mode: None,
            reply_markup: None,
        };

        self.send_telegram_request("sendMessage", &request).await
    }

    async fn send_message_markdown(&self, chat_id: i64, text: &str) -> Result<(), BotError> {
        let request = SendMessageRequest {
            chat_id,
            text: text.to_string(),
            parse_mode: Some("Markdown".to_string()),
            reply_markup: None,
        };

        self.send_telegram_request("sendMessage", &request).await
    }

    async fn send_message_with_keyboard(
        &self,
        chat_id: i64,
        text: &str,
        keyboard: InlineKeyboardMarkup,
    ) -> Result<(), BotError> {
        let request = SendMessageRequest {
            chat_id,
            text: text.to_string(),
            parse_mode: Some("Markdown".to_string()),
            reply_markup: Some(keyboard),
        };

        self.send_telegram_request("sendMessage", &request).await
    }

    async fn answer_callback_query(&self, callback_id: &str, text: Option<&str>) -> Result<(), BotError> {
        let mut params = vec![("callback_query_id", callback_id)];
        if let Some(t) = text {
            params.push(("text", t));
        }

        let url = format!("https://api.telegram.org/bot{}/answerCallbackQuery", self.token);
        self.http_client
            .post(&url)
            .form(&params)
            .send()
            .await?;

        Ok(())
    }

    async fn send_telegram_request<T: Serialize>(&self, method: &str, request: &T) -> Result<(), BotError> {
        let url = format!("https://api.telegram.org/bot{}/{}", self.token, method);
        
        let response = self.http_client
            .post(&url)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(BotError::TelegramApiError(response.text().await?));
        }

        Ok(())
    }

    fn create_slug(question: &str) -> String {
        question
            .to_lowercase()
            .replace('?', "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-")
            .chars()
            .take(50)
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BotError {
    #[error("No text in message")]
    NoText,
    #[error("Invalid amount")]
    InvalidAmount,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Telegram API error: {0}")]
    TelegramApiError(String),
}
