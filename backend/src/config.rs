// ============================================================
// FILE: config.rs
// PURPOSE: Defines and loads all runtime configuration for the backend.
//          All configuration comes from environment variables (12-factor app pattern).
//          This means no hardcoded values - everything is configurable per environment.
//
// USAGE:
//   Called once in main.rs: let config = Config::from_env()?;
//   Then stored in AppState and accessed via state.config()
//
// ENVIRONMENT VARIABLES:
//   Required (will fail to start if missing):
//     - DATABASE_URL: PostgreSQL connection string
//     - JWT_SECRET: Secret key for signing/verifying JWT tokens
//
//   Optional (have sensible defaults):
//     - REDIS_URL: Redis connection string (default: redis://127.0.0.1:6379)
//     - CORS_ORIGINS: Comma-separated allowed origins (default: localhost:5173,3000)
//     - RUST_LOG: Log level filter (default: backend=debug,tower_http=debug)
//     - RUST_PORT: Server port (default: 8000)
//     - FRONTEND_URL: Frontend URL for CORS (default: http://localhost:5173)
//     - And many more for M-Pesa, email, Web3Auth...
//
// JUNIOR DEV NOTE:
//   The .context("...") calls provide helpful error messages when required
//   env vars are missing. e.g., "DATABASE_URL must be set"
// ============================================================

// anyhow: flexible error handling library
// Context: adds context messages to errors (e.g., "DATABASE_URL must be set")
// Result: anyhow's Result type (can hold any error)
use anyhow::{Context, Result};

// ============================================================
// STRUCT: Config
// PURPOSE: Holds all runtime configuration values.
//          Derived Debug: allows printing the config for debugging (be careful with secrets!)
//          Derived Clone: allows the config to be cheaply copied (it's stored in Arc<Inner>)
// ============================================================
#[derive(Debug, Clone)]
pub struct Config {
    // ---- REQUIRED FIELDS ----

    /// PostgreSQL connection string.
    /// Format: postgres://username:password@host:port/database
    /// Example: postgres://polypulse:secret@localhost:5432/polypulse_db
    pub database_url: String,

    /// Redis connection string for caching and session storage.
    /// Format: redis://host:port
    /// Default: redis://127.0.0.1:6379
    pub redis_url: String,

    /// Secret key used to sign and verify JWT tokens.
    /// MUST be kept secret - anyone with this key can forge tokens!
    /// Should be a long random string (at least 32 characters).
    pub jwt_secret: String,

    /// List of allowed CORS origins (frontend URLs that can make requests).
    /// Parsed from comma-separated CORS_ORIGINS env var.
    /// Example: ["http://localhost:5173", "https://polypulse.co.ke"]
    pub cors_origins: Vec<String>,

    /// Log level filter string (passed to tracing_subscriber's EnvFilter).
    /// Example: "backend=debug,tower_http=info"
    pub rust_log: String,

    /// TCP port the server listens on.
    /// Default: 8000
    pub port: u16,

    /// The frontend application URL (used for CORS and redirect URLs).
    /// Default: http://localhost:5173
    pub frontend_url: String,

    // ---- OPTIONAL FIELDS ----
    // These use Option<String> because they may not be configured in all environments.

    /// Web3Auth client ID for social login (legacy feature, may be removed).
    pub web3auth_client_id: Option<String>,

    // ---- M-PESA / DARAJA API FIELDS ----
    // M-Pesa is Kenya's mobile money service. Daraja is Safaricom's API for it.
    // These are only needed if M-Pesa deposits are enabled.

    /// M-Pesa environment: "sandbox" for testing, "production" for real money
    pub mpesa_env: Option<String>,

    /// Daraja API consumer key (like an API username)
    pub mpesa_consumer_key: Option<String>,

    /// Daraja API consumer secret (like an API password)
    pub mpesa_consumer_secret: Option<String>,

    /// M-Pesa business shortcode (the "till number" or "paybill number")
    pub mpesa_shortcode: Option<String>,

    /// M-Pesa passkey for generating STK push passwords
    pub mpesa_passkey: Option<String>,

    /// URL that Safaricom calls back with payment confirmation
    pub mpesa_callback_url: Option<String>,

    // ---- EMAIL / SMTP FIELDS ----
    // Used for sending transactional emails (notifications, password resets, etc.)

    /// SMTP server hostname (e.g., "smtp.gmail.com")
    pub email_host: Option<String>,

    /// SMTP server port (default: 587 for TLS/STARTTLS)
    pub email_port: u16,

    /// SMTP authentication username (usually the email address)
    pub email_user: Option<String>,

    /// SMTP authentication password or app password
    pub email_password: Option<String>,

    /// The "From" address for outgoing emails
    /// Default: noreply@polypulse.co.ke
    pub email_from: String,
} // end Config struct

// ============================================================
// IMPLEMENTATION: Config
// ============================================================
impl Config {
    // ============================================================
    // FUNCTION: from_env
    // PURPOSE: Reads all configuration from environment variables.
    //          Required variables use .context() to provide helpful error messages.
    //          Optional variables use .ok() to return None if not set.
    //          Called once at startup in main.rs.
    // RETURNS: Result<Config> - Ok(Config) if all required vars are set, Err otherwise
    // ============================================================
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            // DATABASE_URL is required - fail fast with a clear error if missing
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,

            // REDIS_URL is optional - default to local Redis if not set
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),

            // JWT_SECRET is required - without it we can't authenticate users
            jwt_secret: std::env::var("JWT_SECRET")
                .context("JWT_SECRET must be set")?,

            // CORS_ORIGINS: parse comma-separated string into a Vec<String>
            // .split(',') splits on commas
            // .map(|s| s.trim().to_string()) removes whitespace around each origin
            // .collect() gathers into a Vec
            cors_origins: std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173,http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),

            // RUST_LOG: log level filter for tracing_subscriber
            rust_log: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".to_string()),

            // RUST_PORT: parse string to u16 (port number)
            // .parse() converts "8000" to 8000u16
            // .context() provides a helpful error if the value is not a valid port
            port: std::env::var("RUST_PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .context("RUST_PORT must be a valid port number")?,

            // FRONTEND_URL: used for CORS configuration
            frontend_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),

            // Web3Auth: optional, .ok() converts Err to None
            web3auth_client_id: std::env::var("WEB3AUTH_CLIENT_ID").ok(),

            // M-Pesa: all optional
            mpesa_env: std::env::var("MPESA_ENV").ok(),
            mpesa_consumer_key: std::env::var("MPESA_CONSUMER_KEY").ok(),
            mpesa_consumer_secret: std::env::var("MPESA_CONSUMER_SECRET").ok(),
            mpesa_shortcode: std::env::var("MPESA_SHORTCODE").ok(),
            mpesa_passkey: std::env::var("MPESA_PASSKEY").ok(),
            mpesa_callback_url: std::env::var("MPESA_CALLBACK_URL").ok(),

            // Email: host, user, password are optional; port and from have defaults
            email_host: std::env::var("EMAIL_HOST").ok(),
            email_port: std::env::var("EMAIL_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),  // If parsing fails, default to 587 (standard SMTP TLS port)
            email_user: std::env::var("EMAIL_HOST_USER").ok(),
            email_password: std::env::var("EMAIL_HOST_PASSWORD").ok(),
            email_from: std::env::var("DEFAULT_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@polypulse.co.ke".to_string()),
        })
    } // end from_env
} // end impl Config
