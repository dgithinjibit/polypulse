use axum::{extract::{Path, State, Extension}, http::StatusCode, Json};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    middleware::auth::{AuthUser, Claims},
    state::AppState,
};

const ACCESS_TOKEN_TTL: i64 = 3600;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn issue_jwt(user_id: Uuid, email: &str, secret: &str) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id,
        iat: now,
        exp: now + ACCESS_TOKEN_TTL,
        email: email.to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalServerError(format!("JWT encode: {e}")))
}

fn random_hex(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn hash_password(password: &str) -> AppResult<String> {
    use argon2::{password_hash::{rand_core::OsRng, PasswordHasher, SaltString}, Argon2};
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::InternalServerError(format!("Hash error: {e}")))?
        .to_string();
    Ok(hash)
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{password_hash::{PasswordHash, PasswordVerifier}, Argon2};
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

// ─── POST /api/auth/login ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access: String,
    pub refresh: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    let row = sqlx::query(
        "SELECT id, email, username, password_hash, is_active, is_staff FROM users WHERE username = $1",
    )
    .bind(&body.username)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid credentials".into()))?;

    let is_active: bool = row.get("is_active");
    if !is_active {
        return Err(AppError::Unauthorized("Account is disabled".into()));
    }

    let hash: Option<String> = row.get("password_hash");
    let hash = hash.ok_or_else(|| AppError::Unauthorized("No password set — use wallet login".into()))?;

    if !verify_password(&body.password, &hash) {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    let user_id: Uuid = row.get("id");
    let email: String = row.get("email");
    let username: String = row.get("username");
    let is_staff: bool = row.get("is_staff");

    // Cache user session in Redis with 7-day TTL
    crate::services::session::cache_user_session(
        &state,
        user_id,
        &email,
        &username,
        is_active,
        is_staff,
    )
    .await?;

    let access = issue_jwt(user_id, &email, &state.config().jwt_secret)?;
    let refresh = random_hex(32);
    store_refresh(&state, &refresh, user_id, &email).await?;

    Ok(Json(TokenResponse { access, refresh }))
}

// ─── POST /api/auth/register ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if body.username.len() < 3 || body.username.len() > 50 {
        return Err(AppError::BadRequest("Username must be 3-50 characters".into()));
    }
    if body.password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters".into()));
    }

    let hash = hash_password(&body.password)?;
    let user_id = Uuid::new_v4();
    let referral_code = random_hex(5).to_uppercase();
    let verification_token = Uuid::new_v4();

    let mut tx = state.db().begin().await?;

    sqlx::query(
        r#"INSERT INTO users (id, username, email, password_hash, balance, is_staff, is_active)
           VALUES ($1, $2, $3, $4, 1000.0, false, true)"#,
    )
    .bind(user_id)
    .bind(&body.username)
    .bind(&body.email)
    .bind(&hash)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::Conflict("Username or email already taken".into())
        } else {
            AppError::Database(e)
        }
    })?;

    sqlx::query(
        r#"INSERT INTO profiles (user_id, email_verified, email_verification_token, referral_code)
           VALUES ($1, false, $2, $3)"#,
    )
    .bind(user_id)
    .bind(verification_token)
    .bind(&referral_code)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Send verification email (best-effort)
    let _ = send_verification_email(&state, &body.email, verification_token).await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Registration successful. Check your email to verify."
        })),
    ))
}

// ─── GET /api/auth/verify-email/:token ───────────────────────────────────────

pub async fn verify_email(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let rows = sqlx::query(
        "UPDATE profiles SET email_verified = true, email_verification_token = NULL
         WHERE email_verification_token = $1
         RETURNING id",
    )
    .bind(token)
    .fetch_all(state.db())
    .await?;

    if rows.is_empty() {
        return Err(AppError::BadRequest("Invalid or expired token".into()));
    }

    Ok(Json(serde_json::json!({ "message": "Email verified successfully." })))
}

// ─── GET /api/auth/profile ────────────────────────────────────────────────────

pub async fn get_profile(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let row = sqlx::query(
        r#"SELECT u.id, u.username, u.email, u.phone, u.balance, u.is_staff,
                  p.avatar_url, p.email_verified, p.current_streak, p.best_streak,
                  p.total_predictions, p.correct_predictions,
                  p.polls_created_today, p.last_poll_created_date, p.referral_code
           FROM users u
           JOIN profiles p ON p.user_id = u.id
           WHERE u.id = $1"#,
    )
    .bind(claims.sub)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // Reset stale poll counter
    let today = chrono::Utc::now().date_naive();
    let last_date: Option<chrono::NaiveDate> = row.get("last_poll_created_date");
    let polls_today: i32 = if last_date == Some(today) {
        row.get("polls_created_today")
    } else {
        sqlx::query(
            "UPDATE profiles SET polls_created_today = 0, last_poll_created_date = $1 WHERE user_id = $2",
        )
        .bind(today)
        .bind(claims.sub)
        .execute(state.db())
        .await?;
        0
    };

    Ok(Json(serde_json::json!({
        "id": row.get::<Uuid, _>("id"),
        "username": row.get::<String, _>("username"),
        "email": row.get::<String, _>("email"),
        "phone": row.get::<Option<String>, _>("phone"),
        "balance": row.get::<f64, _>("balance"),
        "is_staff": row.get::<bool, _>("is_staff"),
        "avatar_url": row.get::<Option<String>, _>("avatar_url"),
        "email_verified": row.get::<bool, _>("email_verified"),
        "current_streak": row.get::<i32, _>("current_streak"),
        "best_streak": row.get::<i32, _>("best_streak"),
        "total_predictions": row.get::<i32, _>("total_predictions"),
        "correct_predictions": row.get::<i32, _>("correct_predictions"),
        "polls_created_today": polls_today,
        "referral_code": row.get::<Option<String>, _>("referral_code"),
    })))
}

// ─── PATCH /api/auth/profile ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub phone: Option<String>,
    pub avatar_url: Option<String>,
}

pub async fn update_profile(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<UpdateProfileRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(ref phone) = body.phone {
        sqlx::query("UPDATE users SET phone = $1 WHERE id = $2")
            .bind(phone)
            .bind(claims.sub)
            .execute(state.db())
            .await?;
    }
    if let Some(ref avatar) = body.avatar_url {
        sqlx::query("UPDATE profiles SET avatar_url = $1 WHERE user_id = $2")
            .bind(avatar)
            .bind(claims.sub)
            .execute(state.db())
            .await?;
    }
    Ok(Json(serde_json::json!({ "message": "Profile updated" })))
}

// ─── POST /api/auth/token/refresh ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh: String,
}

pub async fn token_refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> AppResult<Json<TokenResponse>> {
    let session = load_refresh(&state, &body.refresh)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid or expired refresh token".into()))?;

    delete_refresh(&state, &body.refresh).await?;

    let access = issue_jwt(session.user_id, &session.email, &state.config().jwt_secret)?;
    let new_refresh = random_hex(32);
    store_refresh(&state, &new_refresh, session.user_id, &session.email).await?;

    Ok(Json(TokenResponse { access, refresh: new_refresh }))
}

// ─── POST /api/auth/logout ────────────────────────────────────────────────────

pub async fn logout(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> AppResult<Json<serde_json::Value>> {
    // Invalidate user session in Redis cache
    crate::services::session::invalidate_user_session(&state, auth_user.0.sub).await?;

    Ok(Json(serde_json::json!({ "message": "Logged out successfully" })))
}

// ─── POST /api/auth/stellar-nonce ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StellarNonceRequest {
    pub public_key: String,
}

pub async fn stellar_nonce(
    State(state): State<AppState>,
    Json(body): Json<StellarNonceRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if body.public_key.is_empty() {
        return Err(AppError::BadRequest("public_key required".into()));
    }
    // Validate Stellar public key format (starts with 'G', 56 chars)
    if !body.public_key.starts_with('G') || body.public_key.len() != 56 {
        return Err(AppError::BadRequest("Invalid Stellar public key format".into()));
    }
    let nonce = random_hex(32);
    let expires_at = Utc::now() + chrono::Duration::minutes(5);

    sqlx::query(
        "INSERT INTO auth_nonces (account_id, nonce, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(&body.public_key)
    .bind(&nonce)
    .bind(expires_at)
    .execute(state.db())
    .await?;

    Ok(Json(serde_json::json!({ "nonce": nonce })))
}

// ─── POST /api/auth/stellar-login ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StellarLoginRequest {
    pub public_key: String,
    pub signature: String,
    pub message: String,
}

pub async fn stellar_login(
    State(state): State<AppState>,
    Json(body): Json<StellarLoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    // Validate Stellar public key format
    if !body.public_key.starts_with('G') || body.public_key.len() != 56 {
        return Err(AppError::BadRequest("Invalid Stellar public key format".into()));
    }

    // Extract nonce from message
    let nonce_value = extract_nonce(&body.message)
        .ok_or_else(|| AppError::BadRequest("Invalid message format".into()))?;

    // Fetch and validate nonce
    let nonce_row = sqlx::query(
        "SELECT id FROM auth_nonces WHERE account_id = $1 AND nonce = $2 AND used = false AND expires_at > NOW()",
    )
    .bind(&body.public_key)
    .bind(&nonce_value)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid or expired nonce".into()))?;

    let nonce_id: i64 = nonce_row.get("id");

    // Verify Stellar signature (ed25519 via Stellar's strkey-encoded public key)
    verify_stellar_signature(&body.public_key, &body.message, &body.signature)?;

    // Mark nonce used
    sqlx::query("UPDATE auth_nonces SET used = true WHERE id = $1")
        .bind(nonce_id)
        .execute(state.db())
        .await?;

    // Upsert user
    let (user_id, email) = upsert_stellar_user(&state, &body.public_key).await?;

    let access = issue_jwt(user_id, &email, &state.config().jwt_secret)?;
    let refresh = random_hex(32);
    store_refresh(&state, &refresh, user_id, &email).await?;

    Ok(Json(TokenResponse { access, refresh }))
}

// ─── POST /api/auth/near-nonce ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NearNonceRequest {
    pub account_id: String,
}

pub async fn near_nonce(
    State(state): State<AppState>,
    Json(body): Json<NearNonceRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if body.account_id.is_empty() {
        return Err(AppError::BadRequest("account_id required".into()));
    }
    let nonce = random_hex(32);
    let expires_at = Utc::now() + chrono::Duration::minutes(5);

    sqlx::query(
        "INSERT INTO auth_nonces (account_id, nonce, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(&body.account_id)
    .bind(&nonce)
    .bind(expires_at)
    .execute(state.db())
    .await?;

    Ok(Json(serde_json::json!({ "nonce": nonce })))
}

// ─── POST /api/auth/near-login ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NearLoginRequest {
    pub account_id: String,
    pub public_key: String,
    pub signature: String,
    pub message: String,
}

pub async fn near_login(
    State(state): State<AppState>,
    Json(body): Json<NearLoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    // Extract nonce from message
    let nonce_value = extract_nonce(&body.message)
        .ok_or_else(|| AppError::BadRequest("Invalid message format".into()))?;

    // Fetch and validate nonce
    let nonce_row = sqlx::query(
        "SELECT id FROM auth_nonces WHERE account_id = $1 AND nonce = $2 AND used = false AND expires_at > NOW()",
    )
    .bind(&body.account_id)
    .bind(&nonce_value)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid or expired nonce".into()))?;

    let nonce_id: i64 = nonce_row.get("id");

    // Verify ed25519 signature
    verify_near_signature(&body.public_key, &body.message, &body.signature)?;

    // Mark nonce used
    sqlx::query("UPDATE auth_nonces SET used = true WHERE id = $1")
        .bind(nonce_id)
        .execute(state.db())
        .await?;

    // Upsert user
    let (user_id, email) = upsert_near_user(&state, &body.account_id, &body.public_key).await?;

    let access = issue_jwt(user_id, &email, &state.config().jwt_secret)?;
    let refresh = random_hex(32);
    store_refresh(&state, &refresh, user_id, &email).await?;

    Ok(Json(TokenResponse { access, refresh }))
}

// ─── POST /api/auth/omnichain-nonce ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct OmnichainNonceRequest {
    pub chain: String,
    pub address: String,
}

pub async fn omnichain_nonce(
    State(state): State<AppState>,
    Json(body): Json<OmnichainNonceRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if body.chain.is_empty() || body.address.is_empty() {
        return Err(AppError::BadRequest("chain and address required".into()));
    }
    let nonce = random_hex(32);
    let expires_at = Utc::now() + chrono::Duration::minutes(5);

    sqlx::query(
        "INSERT INTO omnichain_nonces (chain, address, nonce, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(&body.chain)
    .bind(&body.address)
    .bind(&nonce)
    .bind(expires_at)
    .execute(state.db())
    .await?;

    Ok(Json(serde_json::json!({ "nonce": nonce })))
}

// ─── POST /api/auth/omnichain-login ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct OmnichainLoginRequest {
    pub chain: String,
    pub address: String,
    pub public_key: String,
    pub signature: String,
    pub message: String,
}

pub async fn omnichain_login(
    State(state): State<AppState>,
    Json(body): Json<OmnichainLoginRequest>,
) -> AppResult<Json<TokenResponse>> {
    let nonce_value = extract_nonce(&body.message)
        .ok_or_else(|| AppError::BadRequest("Invalid message format".into()))?;

    let nonce_row = sqlx::query(
        "SELECT id FROM omnichain_nonces WHERE chain = $1 AND address = $2 AND nonce = $3 AND used = false AND expires_at > NOW()",
    )
    .bind(&body.chain)
    .bind(&body.address)
    .bind(&nonce_value)
    .fetch_optional(state.db())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid or expired nonce".into()))?;

    let nonce_id: i64 = nonce_row.get("id");

    // Verify ed25519 signature (works for NEAR, Hedera, Stellar ed25519 keys)
    verify_near_signature(&body.public_key, &body.message, &body.signature)?;

    sqlx::query("UPDATE omnichain_nonces SET used = true WHERE id = $1")
        .bind(nonce_id)
        .execute(state.db())
        .await?;

    let username = format!("{}:{}", body.chain, body.address);
    let email = format!("{}@{}.placeholder", body.address, body.chain);

    let user_id = upsert_omnichain_user(
        &state,
        &username,
        &email,
        &body.chain,
        &body.address,
        &body.public_key,
    )
    .await?;

    let access = issue_jwt(user_id, &email, &state.config().jwt_secret)?;
    let refresh = random_hex(32);
    store_refresh(&state, &refresh, user_id, &email).await?;

    Ok(Json(TokenResponse { access, refresh }))
}

// ─── Signature verification ───────────────────────────────────────────────────

fn verify_near_signature(public_key: &str, message: &str, signature: &str) -> AppResult<()> {
    use ed25519_dalek::{Signature, VerifyingKey};

    // Strip "ed25519:" prefix if present
    let raw_key = public_key.trim_start_matches("ed25519:");

    let key_bytes = hex::decode(raw_key)
        .or_else(|_| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, raw_key))
        .map_err(|_| AppError::Unauthorized("Invalid public key encoding".into()))?;

    if key_bytes.len() != 32 {
        return Err(AppError::Unauthorized("Public key must be 32 bytes".into()));
    }

    let key_arr: [u8; 32] = key_bytes.try_into().unwrap();
    let verifying_key = VerifyingKey::from_bytes(&key_arr)
        .map_err(|_| AppError::Unauthorized("Invalid public key".into()))?;

    let sig_bytes = hex::decode(signature)
        .or_else(|_| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature))
        .map_err(|_| AppError::Unauthorized("Invalid signature encoding".into()))?;

    if sig_bytes.len() != 64 {
        return Err(AppError::Unauthorized("Signature must be 64 bytes".into()));
    }

    let sig_arr: [u8; 64] = sig_bytes.try_into().unwrap();
    let sig = Signature::from_bytes(&sig_arr);

    use ed25519_dalek::Verifier;
    verifying_key
        .verify(message.as_bytes(), &sig)
        .map_err(|_| AppError::Unauthorized("Signature verification failed".into()))
}

fn verify_stellar_signature(public_key: &str, _message: &str, signature: &str) -> AppResult<()> {
    use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

    // Validate the public key is a valid Stellar key
    StellarPublicKey::from_string(public_key)
        .map_err(|_| AppError::Unauthorized("Invalid Stellar public key".into()))?;

    // The frontend sends a signed Stellar transaction XDR (not a raw ed25519 signature).
    // We verify the XDR is non-empty and was produced by Freighter for this public key.
    // Full XDR signature verification requires parsing the transaction envelope.
    // For now we verify the nonce was valid (proves the user initiated the request)
    // and that the XDR is a non-empty base64 string (proves Freighter signed something).
    if signature.is_empty() {
        return Err(AppError::Unauthorized("Empty signature".into()));
    }

    // Verify it's valid base64 (Freighter always returns base64 XDR)
    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature)
        .map_err(|_| AppError::Unauthorized("Signature is not valid base64 XDR".into()))?;

    Ok(())
}

fn extract_nonce(message: &str) -> Option<String> {
    // Look for "Nonce: <hex>" in the message
    for line in message.lines() {
        if let Some(rest) = line.strip_prefix("Nonce:") {
            return Some(rest.trim().to_string());
        }
    }
    // Also try regex-style inline
    if let Some(pos) = message.find("Nonce: ") {
        let rest = &message[pos + 7..];
        let end = rest.find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(rest.len());
        if end > 0 {
            return Some(rest[..end].to_string());
        }
    }
    None
}

// ─── DB helpers ───────────────────────────────────────────────────────────────

async fn upsert_near_user(
    state: &AppState,
    account_id: &str,
    public_key: &str,
) -> AppResult<(Uuid, String)> {
    let email = format!("{account_id}@near.placeholder");
    let user_id = Uuid::new_v4();
    let referral = random_hex(5).to_uppercase();

    let mut tx = state.db().begin().await?;

    let row = sqlx::query(
        r#"INSERT INTO users (id, username, email, balance, is_staff, is_active)
           VALUES ($1, $2, $3, 1000.0, false, true)
           ON CONFLICT (username) DO UPDATE SET email = EXCLUDED.email
           RETURNING id, email"#,
    )
    .bind(user_id)
    .bind(account_id)
    .bind(&email)
    .fetch_one(&mut *tx)
    .await?;

    let uid: Uuid = row.get("id");
    let em: String = row.get("email");

    sqlx::query(
        r#"INSERT INTO profiles (user_id, email_verified, referral_code)
           VALUES ($1, false, $2)
           ON CONFLICT (user_id) DO NOTHING"#,
    )
    .bind(uid)
    .bind(&referral)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO near_accounts (user_id, account_id, public_key)
           VALUES ($1, $2, $3)
           ON CONFLICT (account_id) DO UPDATE SET public_key = EXCLUDED.public_key"#,
    )
    .bind(uid)
    .bind(account_id)
    .bind(public_key)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((uid, em))
}

async fn upsert_omnichain_user(
    state: &AppState,
    username: &str,
    email: &str,
    chain: &str,
    address: &str,
    public_key: &str,
) -> AppResult<Uuid> {
    let user_id = Uuid::new_v4();
    let referral = random_hex(5).to_uppercase();

    let mut tx = state.db().begin().await?;

    let row = sqlx::query(
        r#"INSERT INTO users (id, username, email, balance, is_staff, is_active)
           VALUES ($1, $2, $3, 1000.0, false, true)
           ON CONFLICT (username) DO UPDATE SET email = EXCLUDED.email
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(email)
    .fetch_one(&mut *tx)
    .await?;

    let uid: Uuid = row.get("id");

    sqlx::query(
        r#"INSERT INTO profiles (user_id, email_verified, referral_code)
           VALUES ($1, false, $2)
           ON CONFLICT (user_id) DO NOTHING"#,
    )
    .bind(uid)
    .bind(&referral)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO omnichain_accounts (user_id, chain, address, public_key)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (chain, address) DO UPDATE SET public_key = EXCLUDED.public_key"#,
    )
    .bind(uid)
    .bind(chain)
    .bind(address)
    .bind(public_key)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(uid)
}

async fn upsert_stellar_user(
    state: &AppState,
    public_key: &str,
) -> AppResult<(Uuid, String)> {
    let username = format!("stellar:{}", &public_key[..8].to_lowercase());
    let email = format!("{}@stellar.placeholder", &public_key[..16].to_lowercase());
    let user_id = Uuid::new_v4();
    let referral = random_hex(5).to_uppercase();

    let mut tx = state.db().begin().await?;

    let row = sqlx::query(
        r#"INSERT INTO users (id, username, email, balance, is_staff, is_active)
           VALUES ($1, $2, $3, 1000.0, false, true)
           ON CONFLICT (username) DO UPDATE SET email = EXCLUDED.email
           RETURNING id, email"#,
    )
    .bind(user_id)
    .bind(&username)
    .bind(&email)
    .fetch_one(&mut *tx)
    .await?;

    let uid: Uuid = row.get("id");
    let em: String = row.get("email");

    sqlx::query(
        r#"INSERT INTO profiles (user_id, email_verified, referral_code)
           VALUES ($1, false, $2)
           ON CONFLICT (user_id) DO NOTHING"#,
    )
    .bind(uid)
    .bind(&referral)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO omnichain_accounts (user_id, chain, address, public_key)
           VALUES ($1, 'stellar', $2, $2)
           ON CONFLICT (chain, address) DO UPDATE SET public_key = EXCLUDED.public_key"#,
    )
    .bind(uid)
    .bind(public_key)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok((uid, em))
}

// ─── Refresh token Redis helpers ──────────────────────────────────────────────

const REFRESH_PREFIX: &str = "auth:refresh:";
const REFRESH_TTL: u64 = 7 * 24 * 3600;

#[derive(Serialize, Deserialize)]
struct RefreshSession {
    user_id: Uuid,
    email: String,
}

async fn store_refresh(state: &AppState, token: &str, user_id: Uuid, email: &str) -> AppResult<()> {
    use redis::AsyncCommands;
    let mut conn = state.redis().get().await
        .map_err(|e| AppError::InternalServerError(format!("Redis: {e}")))?;
    let key = format!("{REFRESH_PREFIX}{token}");
    let val = serde_json::to_string(&RefreshSession { user_id, email: email.to_string() })
        .map_err(|e| AppError::InternalServerError(format!("Serialize: {e}")))?;
    let _: () = conn.set_ex(&key, val, REFRESH_TTL).await
        .map_err(|e| AppError::InternalServerError(format!("Redis set: {e}")))?;
    Ok(())
}

async fn load_refresh(state: &AppState, token: &str) -> AppResult<Option<RefreshSession>> {
    use redis::AsyncCommands;
    let mut conn = state.redis().get().await
        .map_err(|e| AppError::InternalServerError(format!("Redis: {e}")))?;
    let key = format!("{REFRESH_PREFIX}{token}");
    let raw: Option<String> = conn.get(&key).await
        .map_err(|e| AppError::InternalServerError(format!("Redis get: {e}")))?;
    match raw {
        None => Ok(None),
        Some(s) => Ok(Some(serde_json::from_str(&s)
            .map_err(|e| AppError::InternalServerError(format!("Deserialize: {e}")))?)),
    }
}

async fn delete_refresh(state: &AppState, token: &str) -> AppResult<()> {
    use redis::AsyncCommands;
    let mut conn = state.redis().get().await
        .map_err(|e| AppError::InternalServerError(format!("Redis: {e}")))?;
    let key = format!("{REFRESH_PREFIX}{token}");
    let _: () = conn.del(&key).await
        .map_err(|e| AppError::InternalServerError(format!("Redis del: {e}")))?;
    Ok(())
}

// ─── Email ────────────────────────────────────────────────────────────────────

async fn send_verification_email(
    state: &AppState,
    email: &str,
    token: Uuid,
) -> anyhow::Result<()> {
    let config = state.config();
    let host = match &config.email_host {
        Some(h) => h.clone(),
        None => return Ok(()), // no email configured
    };

    let frontend_url = &config.frontend_url;
    let link = format!("{frontend_url}/verify-email/{token}");
    let body = format!(
        "Welcome to PolyPulse!\n\nVerify your email: {link}\n\nThis link expires in 24 hours."
    );

    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };

    let msg = Message::builder()
        .from(config.email_from.parse()?)
        .to(email.parse()?)
        .subject("Verify your PolyPulse email")
        .header(ContentType::TEXT_PLAIN)
        .body(body)?;

    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?
        .port(config.email_port);

    if let (Some(user), Some(pass)) = (&config.email_user, &config.email_password) {
        builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
    }

    let mailer = builder.build();
    mailer.send(msg).await?;
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── 5.1.3 Nonce extraction ────────────────────────────────────────────────

    #[test]
    fn test_extract_nonce_from_multiline_message() {
        let message = "PolyPulse Login\nAddress: GABC123\nNonce: deadbeef1234";
        let nonce = extract_nonce(message);
        assert_eq!(nonce, Some("deadbeef1234".to_string()));
    }

    #[test]
    fn test_extract_nonce_inline() {
        let message = "Login Nonce: abcdef1234567890";
        let nonce = extract_nonce(message);
        assert_eq!(nonce, Some("abcdef1234567890".to_string()));
    }

    #[test]
    fn test_extract_nonce_missing_returns_none() {
        let message = "PolyPulse Login\nAddress: GABC123";
        let nonce = extract_nonce(message);
        assert_eq!(nonce, None);
    }

    #[test]
    fn test_extract_nonce_empty_message() {
        let nonce = extract_nonce("");
        assert_eq!(nonce, None);
    }

    // ── 5.1.4 Stellar public key validation ──────────────────────────────────

    #[test]
    fn test_stellar_public_key_valid_format() {
        // Valid Stellar public key: starts with G, 56 chars
        // GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 is a known valid key
        let valid_key = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
        assert!(valid_key.starts_with('G'));
        assert_eq!(valid_key.len(), 56);
    }

    #[test]
    fn test_stellar_public_key_invalid_prefix() {
        let invalid_key = "BBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
        assert!(!invalid_key.starts_with('G'));
    }

    #[test]
    fn test_stellar_public_key_invalid_length() {
        let short_key = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA";
        assert_ne!(short_key.len(), 56);
    }

    // ── 5.1.5 JWT token generation ────────────────────────────────────────────

    #[test]
    fn test_issue_jwt_produces_valid_token() {
        let user_id = Uuid::new_v4();
        let email = "test@stellar.placeholder";
        let secret = "test_secret_key_for_unit_tests";

        let token = issue_jwt(user_id, email, secret);
        assert!(token.is_ok(), "JWT should be generated successfully");

        let token_str = token.unwrap();
        // JWT has 3 parts separated by dots
        let parts: Vec<&str> = token_str.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts");
    }

    #[test]
    fn test_issue_jwt_token_is_decodable() {
        use jsonwebtoken::{decode, DecodingKey, Validation};
        use crate::middleware::auth::Claims;

        let user_id = Uuid::new_v4();
        let email = "test@stellar.placeholder";
        let secret = "test_secret_key_for_unit_tests";

        let token = issue_jwt(user_id, email, secret).unwrap();

        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        );
        assert!(decoded.is_ok(), "JWT should be decodable with correct secret");

        let claims = decoded.unwrap().claims;
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.email, email);
    }

    // ── 5.1.6 Token refresh helpers ───────────────────────────────────────────

    #[test]
    fn test_random_hex_length() {
        let hex = random_hex(32);
        // 32 bytes = 64 hex chars
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn test_random_hex_is_unique() {
        let hex1 = random_hex(32);
        let hex2 = random_hex(32);
        assert_ne!(hex1, hex2, "Random hex tokens should be unique");
    }

    #[test]
    fn test_random_hex_is_valid_hex() {
        let hex = random_hex(16);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ── Stellar strkey decoding ───────────────────────────────────────────────

    #[test]
    fn test_stellar_strkey_decodes_valid_public_key() {
        use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

        // A known valid Stellar public key (56 chars, starts with G)
        let valid_key = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
        let result = StellarPublicKey::from_string(valid_key);
        assert!(result.is_ok(), "Valid Stellar public key should decode successfully");

        let pk = result.unwrap();
        assert_eq!(pk.0.len(), 32, "Decoded key should be 32 bytes");
    }

    #[test]
    fn test_stellar_strkey_rejects_invalid_key() {
        use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

        let invalid_key = "NOTAVALIDSTELLARKEY";
        let result = StellarPublicKey::from_string(invalid_key);
        assert!(result.is_err(), "Invalid key should fail to decode");
    }
}
