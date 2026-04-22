// ============================================================
// FILE: middleware/auth.rs
// PURPOSE: JWT authentication middleware for the Axum web framework.
//          Protects routes by validating the JWT token in the Authorization header.
//          On success, injects the authenticated user's claims into the request
//          extensions so route handlers can access them.
//
// HOW IT WORKS:
//   1. Extract the "Bearer <token>" from the Authorization header
//   2. Decode and validate the JWT token using the JWT_SECRET
//   3. Check Redis cache to verify the user session is still active
//   4. If valid, inject AuthUser into request extensions and continue
//   5. If invalid, return 401 Unauthorized
//
// USAGE in routes/mod.rs:
//   .layer(middleware::from_fn_with_state(state.clone(), mw::auth::require_auth))
//
// USAGE in route handlers:
//   pub async fn my_handler(
//       State(state): State<AppState>,
//       Extension(auth_user): Extension<AuthUser>,  // Injected by this middleware
//   ) -> AppResult<...> {
//       let user_id = auth_user.0.sub;  // The authenticated user's UUID
//   }
//
// JUNIOR DEV NOTE:
//   JWT (JSON Web Token) is a self-contained token that encodes user info.
//   It has 3 parts: header.payload.signature
//   The signature is verified using JWT_SECRET - only our server can create valid tokens.
// ============================================================

// Axum imports for middleware
use axum::{
    extract::{Request, State},  // Request: the incoming HTTP request; State: shared app state
    middleware::Next,            // Next: the next middleware/handler in the chain
    response::Response,          // Response: the HTTP response
};

// jsonwebtoken: JWT decoding and validation
// decode: decodes and validates a JWT token
// DecodingKey: wraps the secret key used for verification
// Validation: configuration for JWT validation (expiry, algorithm, etc.)
use jsonwebtoken::{decode, DecodingKey, Validation};

// serde: serialization/deserialization for the Claims struct
use serde::{Deserialize, Serialize};

// Uuid: universally unique identifier for user IDs
use uuid::Uuid;

// Local imports
use crate::{errors::AppError, state::AppState};

// ============================================================
// STRUCT: Claims
// PURPOSE: The payload embedded inside a JWT token.
//          When we issue a JWT, we encode these fields into it.
//          When we decode a JWT, we get these fields back.
//          The JWT signature ensures these fields haven't been tampered with.
//
// JUNIOR DEV NOTE:
//   "sub" (subject) is a standard JWT claim - it identifies who the token is for.
//   "iat" (issued at) and "exp" (expiry) are standard JWT time claims.
// ============================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user's UUID (primary key in the database)
    /// This is how we identify which user made the request.
    pub sub: Uuid,

    /// Issued-at timestamp (Unix timestamp in seconds)
    /// When the token was created. Used for auditing.
    pub iat: i64,

    /// Expiry timestamp (Unix timestamp in seconds)
    /// After this time, the token is invalid. Currently set to 1 hour from issue.
    pub exp: i64,

    /// User's email address
    /// Embedded in the token so we don't need a DB lookup for basic user info.
    pub email: String,
} // end Claims struct

// ============================================================
// STRUCT: AuthUser
// PURPOSE: A wrapper around Claims that is injected into request extensions.
//          Route handlers extract this to get the authenticated user's info.
//          The tuple struct pattern (AuthUser(pub Claims)) allows easy destructuring.
//
// USAGE in handlers:
//   Extension(auth_user): Extension<AuthUser>
//   let user_id = auth_user.0.sub;
//   let email = &auth_user.0.email;
// ============================================================
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

// ============================================================
// FUNCTION: require_auth
// PURPOSE: Axum middleware function that validates JWT tokens.
//          Applied to protected route groups in routes/mod.rs.
//          If validation passes, the request continues to the handler.
//          If validation fails, returns 401 Unauthorized immediately.
//
// PARAMS:
//   - State(state): Shared app state (needed for JWT secret and Redis)
//   - req: The incoming HTTP request (mutable so we can add extensions)
//   - next: The next middleware or route handler to call if auth passes
// RETURNS: Result<Response, AppError>
//   - Ok(Response): Auth passed, handler ran and returned a response
//   - Err(AppError::Unauthorized): Auth failed, returns 401 to client
// ============================================================
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // STEP 1: Extract the Bearer token from the Authorization header
    // Returns None if header is missing or malformed
    let token = extract_bearer_token(&req)
        .ok_or_else(|| AppError::Unauthorized("Missing or invalid Authorization header".to_string()))?;

    // STEP 2: Decode and validate the JWT token
    // DecodingKey::from_secret: creates a key from our JWT_SECRET bytes
    // Validation::default(): validates expiry (exp), algorithm (HS256), etc.
    // If the token is expired, tampered with, or uses wrong secret → error
    let secret = state.config().jwt_secret.as_bytes();
    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map_err(|e| AppError::Unauthorized(format!("Invalid token: {e}")))?;

    // STEP 3: Check Redis cache for the user session
    // Even if the JWT is valid, the user might have been deactivated or logged out.
    // We check Redis to verify the session is still active.
    // This adds a small overhead but provides better security (immediate revocation).
    let session = crate::services::session::get_user_session(&state, token_data.claims.sub)
        .await
        .map_err(|e| {
            // Log the cache error but don't expose details to the client
            tracing::warn!("Session cache check failed: {e}");
            e
        })?;

    // STEP 4: Verify the user account is active
    if let Some(session) = session {
        // Session found - check if account is still active
        if !session.is_active {
            // Account was deactivated - reject even with valid JWT
            return Err(AppError::Unauthorized("Account is disabled".to_string()));
        } // end is_active check
    } else {
        // No session in Redis - user not found or session expired
        // This happens if the user was deleted or their session was invalidated
        return Err(AppError::Unauthorized("User not found".to_string()));
    } // end session check

    // STEP 5: Inject the authenticated user's claims into request extensions
    // This makes AuthUser available to all downstream handlers via Extension<AuthUser>
    req.extensions_mut().insert(AuthUser(token_data.claims));

    // STEP 6: Continue to the next middleware or route handler
    Ok(next.run(req).await)
} // end require_auth

// ============================================================
// FUNCTION: extract_bearer_token
// PURPOSE: Extracts the JWT token string from the Authorization header.
//          The Authorization header format is: "Bearer <token>"
//          This function strips the "Bearer " prefix and returns just the token.
// PARAM req: The incoming HTTP request
// RETURNS: Option<String> - Some(token) if header is valid, None otherwise
// ============================================================
fn extract_bearer_token(req: &Request) -> Option<String> {
    // Get the Authorization header value
    // Returns None if the header doesn't exist
    let header = req.headers().get(axum::http::header::AUTHORIZATION)?;

    // Convert header bytes to a string
    // Returns None if the header contains non-UTF8 bytes
    let value = header.to_str().ok()?;

    // Strip the "Bearer " prefix and return the token
    // Returns None if the header doesn't start with "Bearer "
    value.strip_prefix("Bearer ").map(|s| s.to_string())
} // end extract_bearer_token
