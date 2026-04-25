//! Input validation middleware and utilities.
//!
//! This module provides validation functions for user inputs to ensure data integrity
//! and prevent security vulnerabilities like XSS attacks.

use regex::Regex;
use std::sync::OnceLock;

use crate::errors::AppError;

// ─── Constants ────────────────────────────────────────────────────────────────

/// Minimum username length
pub const MIN_USERNAME_LENGTH: usize = 3;

/// Maximum username length
pub const MAX_USERNAME_LENGTH: usize = 30;

/// Minimum poll options count
pub const MIN_POLL_OPTIONS: usize = 2;

/// Maximum poll options count
pub const MAX_POLL_OPTIONS: usize = 10;

/// Minimum bet amount
pub const MIN_BET_AMOUNT: f64 = 1.0;

// ─── Phone Number Validation ──────────────────────────────────────────────────

/// Returns a compiled regex for validating phone numbers.
/// Supports international format with optional + prefix and country code.
fn phone_regex() -> &'static Regex {
    static PHONE_REGEX: OnceLock<Regex> = OnceLock::new();
    PHONE_REGEX.get_or_init(|| {
        // Matches: +254712345678, 254712345678, 0712345678
        Regex::new(r"^\+?[0-9]{10,15}$").unwrap()
    })
}

/// Validates a phone number format.
///
/// # Arguments
/// * `phone` - The phone number to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(AppError::BadRequest)` if invalid
pub fn validate_phone(phone: &str) -> Result<(), AppError> {
    if !phone_regex().is_match(phone) {
        return Err(AppError::BadRequest(
            "Invalid phone number format. Expected 10-15 digits with optional + prefix".to_string(),
        ));
    }
    Ok(())
}

// ─── Username Validation ──────────────────────────────────────────────────────

/// Validates a username length.
///
/// # Arguments
/// * `username` - The username to validate
///
/// # Returns
/// * `Ok(())` if valid (3-30 characters)
/// * `Err(AppError::BadRequest)` if invalid
pub fn validate_username(username: &str) -> Result<(), AppError> {
    let len = username.len();
    if len < MIN_USERNAME_LENGTH || len > MAX_USERNAME_LENGTH {
        return Err(AppError::BadRequest(format!(
            "Username must be between {} and {} characters",
            MIN_USERNAME_LENGTH, MAX_USERNAME_LENGTH
        )));
    }
    Ok(())
}

// ─── Email Validation ─────────────────────────────────────────────────────────

/// Returns a compiled regex for validating email addresses.
fn email_regex() -> &'static Regex {
    static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
    EMAIL_REGEX.get_or_init(|| {
        // Basic email validation regex
        Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
    })
}

/// Validates an email format.
///
/// # Arguments
/// * `email` - The email to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(AppError::BadRequest)` if invalid
pub fn validate_email(email: &str) -> Result<(), AppError> {
    if !email_regex().is_match(email) {
        return Err(AppError::BadRequest("Invalid email format".to_string()));
    }
    Ok(())
}

// ─── Bet Amount Validation ────────────────────────────────────────────────────

/// Validates a bet amount.
///
/// # Arguments
/// * `amount` - The bet amount to validate
/// * `user_balance` - The user's current balance
///
/// # Returns
/// * `Ok(())` if valid (positive and within balance)
/// * `Err(AppError::BadRequest)` if invalid
pub fn validate_bet_amount(amount: f64, user_balance: f64) -> Result<(), AppError> {
    if amount < MIN_BET_AMOUNT {
        return Err(AppError::BadRequest(format!(
            "Bet amount must be at least {}",
            MIN_BET_AMOUNT
        )));
    }
    
    if amount > user_balance {
        return Err(AppError::BadRequest(
            "Insufficient balance for this bet".to_string(),
        ));
    }
    
    if !amount.is_finite() {
        return Err(AppError::BadRequest(
            "Bet amount must be a valid number".to_string(),
        ));
    }
    
    Ok(())
}

// ─── Poll Options Validation ──────────────────────────────────────────────────

/// Validates poll options count.
///
/// # Arguments
/// * `options_count` - The number of poll options
///
/// # Returns
/// * `Ok(())` if valid (2-10 options)
/// * `Err(AppError::BadRequest)` if invalid
pub fn validate_poll_options_count(options_count: usize) -> Result<(), AppError> {
    if options_count < MIN_POLL_OPTIONS || options_count > MAX_POLL_OPTIONS {
        return Err(AppError::BadRequest(format!(
            "Poll must have between {} and {} options",
            MIN_POLL_OPTIONS, MAX_POLL_OPTIONS
        )));
    }
    Ok(())
}

// ─── Stellar Address Validation ──────────────────────────────────────────────

/// Validates a Stellar public key (address) format.
///
/// Stellar addresses are 56-character strings starting with 'G' and encoded
/// in base32 (Stellar's strkey format).
///
/// # Arguments
/// * `address` - The Stellar address to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(AppError::BadRequest)` if invalid
pub fn validate_stellar_address(address: &str) -> Result<(), AppError> {
    if address.len() != 56 {
        return Err(AppError::BadRequest(
            "Invalid Stellar address: must be exactly 56 characters".to_string(),
        ));
    }
    if !address.starts_with('G') {
        return Err(AppError::BadRequest(
            "Invalid Stellar address: must start with 'G'".to_string(),
        ));
    }
    // Verify all characters are valid base32 (uppercase alphanumeric, no 0/O/I/L)
    let valid_chars = address.chars().all(|c| {
        matches!(c, 'A'..='Z' | '2'..='7')
    });
    if !valid_chars {
        return Err(AppError::BadRequest(
            "Invalid Stellar address: contains invalid characters".to_string(),
        ));
    }
    Ok(())
}

// ─── Content Sanitization ─────────────────────────────────────────────────────

/// Sanitizes comment content to prevent XSS attacks.
///
/// This function removes or escapes potentially dangerous HTML/JavaScript content.
///
/// # Arguments
/// * `content` - The raw comment content
///
/// # Returns
/// * Sanitized content safe for storage and display
pub fn sanitize_comment_content(content: &str) -> String {
    // Escape special characters in the correct order (& first to avoid double-escaping)
    let content = content
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;");
    
    // Trim whitespace
    content.trim().to_string()
}

/// Validates that comment content is not empty after sanitization.
///
/// # Arguments
/// * `content` - The comment content to validate
///
/// # Returns
/// * `Ok(sanitized_content)` if valid
/// * `Err(AppError::BadRequest)` if empty
pub fn validate_and_sanitize_comment(content: &str) -> Result<String, AppError> {
    let sanitized = sanitize_comment_content(content);
    
    if sanitized.is_empty() {
        return Err(AppError::BadRequest(
            "Comment content cannot be empty".to_string(),
        ));
    }
    
    Ok(sanitized)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_username() {
        // Valid usernames
        assert!(validate_username("abc").is_ok());
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("a".repeat(30).as_str()).is_ok());

        // Invalid usernames
        assert!(validate_username("ab").is_err()); // Too short
        assert!(validate_username("a".repeat(31).as_str()).is_err()); // Too long
    }

    #[test]
    fn test_validate_email() {
        // Valid emails
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user+tag@domain.co.uk").is_ok());

        // Invalid emails
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
    }

    #[test]
    fn test_validate_phone() {
        // Valid phone numbers
        assert!(validate_phone("+254712345678").is_ok());
        assert!(validate_phone("254712345678").is_ok());
        assert!(validate_phone("0712345678").is_ok());
        assert!(validate_phone("+12025551234").is_ok());

        // Invalid phone numbers
        assert!(validate_phone("123").is_err()); // Too short
        assert!(validate_phone("abc123456789").is_err()); // Contains letters
        assert!(validate_phone("+1234567890123456").is_err()); // Too long
    }

    #[test]
    fn test_validate_bet_amount() {
        // Valid bet amounts
        assert!(validate_bet_amount(1.0, 100.0).is_ok());
        assert!(validate_bet_amount(50.0, 100.0).is_ok());
        assert!(validate_bet_amount(100.0, 100.0).is_ok());

        // Invalid bet amounts
        assert!(validate_bet_amount(0.5, 100.0).is_err()); // Below minimum
        assert!(validate_bet_amount(150.0, 100.0).is_err()); // Exceeds balance
        assert!(validate_bet_amount(f64::NAN, 100.0).is_err()); // Not finite
        assert!(validate_bet_amount(f64::INFINITY, 100.0).is_err()); // Not finite
    }

    #[test]
    fn test_validate_poll_options_count() {
        // Valid counts
        assert!(validate_poll_options_count(2).is_ok());
        assert!(validate_poll_options_count(5).is_ok());
        assert!(validate_poll_options_count(10).is_ok());

        // Invalid counts
        assert!(validate_poll_options_count(1).is_err()); // Too few
        assert!(validate_poll_options_count(11).is_err()); // Too many
    }

    #[test]
    fn test_sanitize_comment_content() {
        // XSS attempts
        assert_eq!(
            sanitize_comment_content("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
        
        assert_eq!(
            sanitize_comment_content("<img src=x onerror=alert(1)>"),
            "&lt;img src=x onerror=alert(1)&gt;"
        );

        // Normal content
        assert_eq!(
            sanitize_comment_content("This is a normal comment"),
            "This is a normal comment"
        );

        // Whitespace trimming
        assert_eq!(
            sanitize_comment_content("  trimmed  "),
            "trimmed"
        );
    }

    #[test]
    fn test_validate_and_sanitize_comment() {
        // Valid comment
        let result = validate_and_sanitize_comment("Good comment");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Good comment");

        // Empty after sanitization
        assert!(validate_and_sanitize_comment("   ").is_err());
        assert!(validate_and_sanitize_comment("").is_err());
    }

    #[test]
    fn test_validate_stellar_address() {
        // Valid Stellar address (56 chars, starts with G, valid base32)
        let valid = "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
        assert!(validate_stellar_address(valid).is_ok());

        // Too short
        assert!(validate_stellar_address("GABC123").is_err());

        // Too long
        let too_long = format!("G{}", "A".repeat(56));
        assert!(validate_stellar_address(&too_long).is_err());

        // Doesn't start with G
        let bad_prefix = format!("X{}", "A".repeat(55));
        assert!(validate_stellar_address(&bad_prefix).is_err());

        // Contains invalid base32 characters (0, O, I, L are not in Stellar's alphabet)
        let with_zero = format!("G{}0{}", "A".repeat(27), "A".repeat(27));
        assert!(validate_stellar_address(&with_zero).is_err());

        // Lowercase not valid
        let lowercase = format!("g{}", "a".repeat(55));
        assert!(validate_stellar_address(&lowercase).is_err());
    }
}
