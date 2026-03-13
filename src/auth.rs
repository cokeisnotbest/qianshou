//! Token Authentication Middleware
//!
//! Provides token-based authentication for WebSocket connections.

use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use serde::Deserialize;
use std::sync::Arc;

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    TokenExpired,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing authentication token"),
            AuthError::InvalidToken => write!(f, "Invalid authentication token"),
            AuthError::TokenExpired => write!(f, "Authentication token has expired"),
        }
    }
}

impl std::error::Error for AuthError {}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let message = self.to_string();
        (StatusCode::UNAUTHORIZED, message).into_response()
    }
}

/// Token query parameter from WebSocket upgrade request
#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    pub token: Option<String>,
}

/// Token authentication extractor
///
/// Extracts and validates token from WebSocket upgrade request query string.
/// 
/// # Example
///
/// ```ignore
/// async fn ws_handler(
///     ws: WebSocketUpgrade,
///     auth: TokenAuth,
/// ) -> impl IntoResponse {
///     // auth.token contains the validated token
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TokenAuth {
    pub token: String,
}

/// Token validator trait
pub trait TokenValidator: Send + Sync + Clone {
    fn validate_token(&self, token: &str) -> Result<(), AuthError>;
}

/// Simple token validator that checks against a configured token
#[derive(Clone)]
pub struct SimpleTokenValidator {
    valid_token: Arc<String>,
}

impl SimpleTokenValidator {
    pub fn new(token: String) -> Self {
        Self {
            valid_token: Arc::new(token),
        }
    }
}

impl TokenValidator for SimpleTokenValidator {
    fn validate_token(&self, token: &str) -> Result<(), AuthError> {
        if token.is_empty() {
            return Err(AuthError::MissingToken);
        }
        
        if token != *self.valid_token {
            return Err(AuthError::InvalidToken);
        }
        
        Ok(())
    }
}

/// Auth state that holds the token validator
#[derive(Clone)]
pub struct AuthState {
    validator: SimpleTokenValidator,
}

impl AuthState {
    pub fn new(token: String) -> Self {
        Self {
            validator: SimpleTokenValidator::new(token),
        }
    }

    pub fn validate_token(&self, token: &str) -> Result<(), AuthError> {
        self.validator.validate_token(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        assert_eq!(AuthError::MissingToken.to_string(), "Missing authentication token");
        assert_eq!(AuthError::InvalidToken.to_string(), "Invalid authentication token");
        assert_eq!(AuthError::TokenExpired.to_string(), "Authentication token has expired");
    }

    #[test]
    fn test_auth_error_response() {
        let response = AuthError::MissingToken.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        
        let response = AuthError::InvalidToken.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        
        let response = AuthError::TokenExpired.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_simple_token_validator_valid() {
        let validator = SimpleTokenValidator::new("test-token".to_string());
        assert!(validator.validate_token("test-token").is_ok());
    }

    #[test]
    fn test_simple_token_validator_invalid() {
        let validator = SimpleTokenValidator::new("test-token".to_string());
        assert!(validator.validate_token("wrong-token").is_err());
        assert!(matches!(validator.validate_token("wrong-token").unwrap_err(), AuthError::InvalidToken));
    }

    #[test]
    fn test_simple_token_validator_empty() {
        let validator = SimpleTokenValidator::new("test-token".to_string());
        assert!(validator.validate_token("").is_err());
        assert!(matches!(validator.validate_token("").unwrap_err(), AuthError::MissingToken));
    }

    #[test]
    fn test_auth_state_validate_token() {
        let auth_state = AuthState::new("my-secret-token".to_string());
        assert!(auth_state.validate_token("my-secret-token").is_ok());
        assert!(auth_state.validate_token("wrong-token").is_err());
    }

    #[test]
    fn test_auth_state_clone() {
        let auth_state = AuthState::new("test-token".to_string());
        let _ = auth_state.clone();
    }

    #[tokio::test]
    async fn test_token_query_with_valid_token() {
        // Test the query parsing logic
        let query: TokenQuery = serde_urlencoded::from_str("token=valid-token").unwrap();
        assert_eq!(query.token, Some("valid-token".to_string()));
    }

    #[tokio::test]
    async fn test_token_query_with_missing_token() {
        let query: TokenQuery = serde_urlencoded::from_str("").unwrap();
        assert_eq!(query.token, None);
    }

    #[tokio::test]
    async fn test_token_query_with_empty_token() {
        let query: TokenQuery = serde_urlencoded::from_str("token=").unwrap();
        assert_eq!(query.token, Some("".to_string()));
    }
}
