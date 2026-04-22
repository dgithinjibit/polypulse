//! Web3Auth JWT verification service.
//!
//! Fetches the JWKS from Web3Auth's public endpoint, caches it in Redis for
//! 1 hour, and verifies incoming Web3Auth JWTs.  On success it returns the
//! verified claims so the caller can extract `email` and `wallet_address`.

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    DecodingKey, Validation,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::state::AppState;

/// The public JWKS endpoint published by Web3Auth / OpenLogin.
const WEB3AUTH_JWKS_URL: &str = "https://api.openlogin.com/jwks";

/// Redis key used to cache the raw JWKS JSON.
const JWKS_CACHE_KEY: &str = "web3auth:jwks";

/// Cache TTL in seconds (1 hour).
const JWKS_CACHE_TTL: u64 = 3600;

/// Claims extracted from a verified Web3Auth JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Web3AuthClaims {
    /// Subject — typically the wallet address or provider-specific ID.
    pub sub: String,
    /// User email from the OAuth provider.
    pub email: Option<String>,
    /// Display name from the OAuth provider.
    pub name: Option<String>,
    /// Profile picture URL.
    pub picture: Option<String>,
    /// Wallet address derived by Web3Auth (may be in `wallets` array).
    pub wallet_address: Option<String>,
    /// Issued-at timestamp.
    pub iat: Option<i64>,
    /// Expiry timestamp.
    pub exp: Option<i64>,
    /// Audience — should match the Web3Auth client ID.
    pub aud: Option<serde_json::Value>,
    /// Issuer.
    pub iss: Option<String>,
    /// Aggregated verifier info (Web3Auth-specific).
    pub aggregate_verifier: Option<String>,
    /// Verifier identifier.
    pub verifier: Option<String>,
    /// Verifier ID (e.g. email for Google).
    pub verifier_id: Option<String>,
    /// Wallets array — Web3Auth embeds wallet info here.
    pub wallets: Option<Vec<WalletEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEntry {
    pub public_key: Option<String>,
    pub r#type: Option<String>,
    pub curve: Option<String>,
}

impl Web3AuthClaims {
    /// Returns the best available email, falling back to verifier_id when the
    /// OAuth provider is Google/Apple (which always uses email as verifier_id).
    pub fn resolved_email(&self) -> Option<String> {
        self.email
            .clone()
            .or_else(|| self.verifier_id.clone())
    }

    /// Returns the wallet public key from the `wallets` array if present.
    pub fn resolved_wallet_address(&self) -> Option<String> {
        self.wallet_address.clone().or_else(|| {
            self.wallets
                .as_ref()?
                .iter()
                .find_map(|w| w.public_key.clone())
        })
    }
}

// ---------------------------------------------------------------------------
// JWKS fetching & caching
// ---------------------------------------------------------------------------

/// Fetch the JWKS, using Redis as a cache to avoid hammering the endpoint.
async fn fetch_jwks(state: &AppState) -> Result<JwkSet> {
    // Try Redis cache first
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let cached: Option<String> = conn
        .get(JWKS_CACHE_KEY)
        .await
        .unwrap_or(None);

    if let Some(raw) = cached {
        debug!("JWKS cache hit");
        let jwks: JwkSet = serde_json::from_str(&raw).context("Failed to parse cached JWKS")?;
        return Ok(jwks);
    }

    // Cache miss — fetch from Web3Auth
    info!("Fetching Web3Auth JWKS from {WEB3AUTH_JWKS_URL}");
    let response = reqwest::get(WEB3AUTH_JWKS_URL)
        .await
        .context("Failed to fetch Web3Auth JWKS")?;

    let raw = response
        .text()
        .await
        .context("Failed to read JWKS response body")?;

    // Store in Redis with TTL
    let _: () = conn
        .set_ex(JWKS_CACHE_KEY, &raw, JWKS_CACHE_TTL)
        .await
        .unwrap_or(());

    let jwks: JwkSet = serde_json::from_str(&raw).context("Failed to parse JWKS")?;
    Ok(jwks)
}

// ---------------------------------------------------------------------------
// JWT verification
// ---------------------------------------------------------------------------

/// Verify a Web3Auth-issued JWT and return the decoded claims.
///
/// Steps:
/// 1. Decode the JWT header to get the `kid`.
/// 2. Fetch (or load from cache) the JWKS.
/// 3. Find the matching JWK by `kid`.
/// 4. Verify the signature and standard claims.
pub async fn verify_web3auth_jwt(
    state: &AppState,
    token: &str,
) -> Result<Web3AuthClaims> {
    // 1. Peek at the header to get the key ID
    let header = decode_header(token).context("Failed to decode JWT header")?;
    let kid = header.kid.ok_or_else(|| anyhow!("JWT header missing 'kid'"))?;

    // 2. Fetch JWKS (cached)
    let jwks = fetch_jwks(state).await?;

    // 3. Find the matching key
    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| anyhow!("No JWK found for kid '{kid}'"))?;

    // 4. Build the decoding key from the JWK
    let decoding_key = match &jwk.algorithm {
        AlgorithmParameters::RSA(rsa) => {
            DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
                .context("Failed to build RSA decoding key")?
        }
        AlgorithmParameters::EllipticCurve(ec) => {
            DecodingKey::from_ec_components(&ec.x, &ec.y)
                .context("Failed to build EC decoding key")?
        }
        other => return Err(anyhow!("Unsupported JWK algorithm: {other:?}")),
    };

    // 5. Build validation — Web3Auth tokens use RS256 or ES256
    let mut validation = Validation::new(header.alg);
    // Web3Auth sets aud to the client ID; skip audience check if not configured
    if let Some(client_id) = &state.config().web3auth_client_id {
        validation.set_audience(&[client_id]);
    } else {
        validation.validate_aud = false;
    }

    // 6. Decode & verify
    let token_data = decode::<Web3AuthClaims>(token, &decoding_key, &validation)
        .context("Web3Auth JWT verification failed")?;

    // 7. Sanity-check expiry (jsonwebtoken already does this, but be explicit)
    if let Some(exp) = token_data.claims.exp {
        if exp < Utc::now().timestamp() {
            return Err(anyhow!("Web3Auth JWT has expired"));
        }
    }

    Ok(token_data.claims)
}

// ---------------------------------------------------------------------------
// Invalidate JWKS cache (useful for key rotation)
// ---------------------------------------------------------------------------

/// Force-evict the cached JWKS so the next request re-fetches it.
pub async fn invalidate_jwks_cache(state: &AppState) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;
    let _: () = conn.del(JWKS_CACHE_KEY).await?;
    Ok(())
}
