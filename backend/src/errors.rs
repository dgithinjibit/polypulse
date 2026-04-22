use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error")]
    Serialization(#[from] serde_json::Error),
}

impl AppError {
    /// Log the error with request context if available.
    /// 
    /// This method extracts the request ID from the request extensions
    /// and logs the error with appropriate severity and context.
    fn log_with_context(&self, request: Option<&Request>) {
        // Extract request ID if available
        let request_id = request
            .and_then(|req| req.extensions().get::<String>())
            .map(|id| id.as_str())
            .unwrap_or("unknown");

        // Extract request method and path if available
        let method = request.map(|req| req.method().as_str()).unwrap_or("unknown");
        let path = request.map(|req| req.uri().path()).unwrap_or("unknown");

        match self {
            // Client errors (4xx) - log at debug level
            AppError::BadRequest(msg) => {
                tracing::debug!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Bad request"
                );
            }
            AppError::Unauthorized(msg) => {
                tracing::warn!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Unauthorized access attempt"
                );
            }
            AppError::Forbidden(msg) => {
                tracing::warn!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Forbidden access attempt"
                );
            }
            AppError::NotFound(msg) => {
                tracing::debug!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Resource not found"
                );
            }
            AppError::Conflict(msg) => {
                tracing::debug!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Conflict"
                );
            }
            AppError::UnprocessableEntity(msg) => {
                tracing::debug!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Unprocessable entity"
                );
            }
            AppError::TooManyRequests(msg) => {
                tracing::warn!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Rate limit exceeded"
                );
            }
            // Server errors (5xx) - log at error level with sanitized details
            AppError::InternalServerError(msg) => {
                tracing::error!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %msg,
                    "Internal server error"
                );
            }
            AppError::Internal(e) => {
                tracing::error!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %format!("{:#}", e),
                    "Internal error"
                );
            }
            AppError::Database(e) => {
                tracing::error!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %e,
                    error_code = ?e.as_database_error().map(|de| de.code()),
                    "Database error"
                );
            }
            AppError::Serialization(e) => {
                tracing::error!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    error = %e,
                    "Serialization error"
                );
            }
        }
    }

    /// Get the HTTP status code for this error.
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::InternalServerError(_) 
            | AppError::Internal(_) 
            | AppError::Database(_) 
            | AppError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get a sanitized error message safe to send to clients.
    /// 
    /// This method ensures that internal error details, stack traces,
    /// and sensitive information are not exposed to clients.
    fn sanitized_message(&self) -> String {
        match self {
            // Client errors - return the actual message (already safe)
            AppError::NotFound(msg) 
            | AppError::Unauthorized(msg) 
            | AppError::Forbidden(msg) 
            | AppError::BadRequest(msg) 
            | AppError::Conflict(msg) 
            | AppError::UnprocessableEntity(msg) 
            | AppError::TooManyRequests(msg) => msg.clone(),
            
            // Server errors - return generic messages (hide internal details)
            AppError::InternalServerError(_) => "Internal server error".to_string(),
            AppError::Internal(_) => "Internal server error".to_string(),
            AppError::Database(_) => "Database error occurred".to_string(),
            AppError::Serialization(_) => "Data processing error".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log error with context (request context not available here, logged without it)
        self.log_with_context(None);

        let status = self.status_code();
        let message = self.sanitized_message();

        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_error_status_codes() {
        // Test client errors (4xx)
        assert_eq!(
            AppError::BadRequest("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Unauthorized("test".to_string()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::Forbidden("test".to_string()).status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::NotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::Conflict("test".to_string()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::TooManyRequests("test".to_string()).status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );

        // Test server errors (5xx)
        assert_eq!(
            AppError::InternalServerError("test".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::Internal(anyhow::anyhow!("test")).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_sanitized_messages() {
        // Client errors should return the actual message
        assert_eq!(
            AppError::BadRequest("Invalid input".to_string()).sanitized_message(),
            "Invalid input"
        );
        assert_eq!(
            AppError::NotFound("User not found".to_string()).sanitized_message(),
            "User not found"
        );

        // Server errors should return generic messages (no internal details)
        assert_eq!(
            AppError::InternalServerError("Detailed internal error".to_string()).sanitized_message(),
            "Internal server error"
        );
        assert_eq!(
            AppError::Internal(anyhow::anyhow!("Stack trace here")).sanitized_message(),
            "Internal server error"
        );
        assert_eq!(
            AppError::Database(sqlx::Error::RowNotFound).sanitized_message(),
            "Database error occurred"
        );
        assert_eq!(
            AppError::Serialization(serde_json::from_str::<i32>("invalid").unwrap_err()).sanitized_message(),
            "Data processing error"
        );
    }

    #[test]
    fn test_no_stack_traces_in_client_response() {
        // Verify that internal errors don't expose stack traces
        let internal_error = AppError::Internal(anyhow::anyhow!("Internal error with stack trace"));
        let message = internal_error.sanitized_message();
        
        // Should not contain any internal details
        assert!(!message.contains("stack trace"));
        assert!(!message.contains("Internal error"));
        assert_eq!(message, "Internal server error");
    }

    #[test]
    fn test_database_error_sanitization() {
        // Database errors should not expose SQL details
        let db_error = AppError::Database(sqlx::Error::RowNotFound);
        let message = db_error.sanitized_message();
        
        // Should be generic
        assert_eq!(message, "Database error occurred");
        assert!(!message.contains("RowNotFound"));
    }
}
