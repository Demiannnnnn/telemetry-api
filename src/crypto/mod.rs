//! # Crypto Module
//!
//! Provides cryptographic primitives for:
//! 1. Secure password hashing and verification via bcrypt (cost 12).
//! 2. Password complexity validation.
//! 3. JWT token generation and verification.
//! 4. SHA-256 token hashing for refresh token rotation.
//!
//! # Critical Zero-Knowledge Notice
//! This module NEVER handles decryption of telemetry payloads. Telemetry is end-to-end
//! encrypted on the client side and the server stores only opaque binary blobs.

use crate::errors::AppError;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// User roles supported by the authentication system.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserRole {
    /// System Administrator role.
    Admin,
    /// Monitored Worker role.
    Worker,
}

/// JWT Claims payload.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Claims {
    /// Subject identifier (Admin ID or Worker ID).
    pub sub: Uuid,
    /// User role.
    pub role: UserRole,
    /// User email address.
    pub email: String,
    /// Expiration timestamp in seconds since UNIX epoch.
    pub exp: usize,
    /// Issued-at timestamp in seconds since UNIX epoch.
    pub iat: usize,
    /// Issuer ("telemetry-api").
    pub iss: String,
}

/// Hashes a plaintext password using bcrypt with cost factor 12.
///
/// # Arguments
///
/// * `password` - Plaintext password to hash
///
/// # Errors
///
/// Returns [`AppError::Internal`] if bcrypt hashing fails.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    bcrypt::hash(password, 12)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing error: {}", e)))
}

/// Verifies a plaintext password against a bcrypt hash in constant time.
///
/// # Arguments
///
/// * `password` - Plaintext password
/// * `hash` - Stored bcrypt hash
///
/// # Errors
///
/// Returns [`AppError::Internal`] if verification encounters an error.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    bcrypt::verify(password, hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Password verification error: {}", e)))
}

/// Validates that a password satisfies the security complexity policy:
/// - 8 to 72 characters length
/// - At least 1 uppercase letter
/// - At least 1 lowercase letter
/// - At least 1 numeric digit
/// - At least 1 special character (`!@#$%^&*()_+-=`)
///
/// # Errors
///
/// Returns [`AppError::Validation`] if requirements are not met.
pub fn validate_password_complexity(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }
    if password.len() > 72 {
        return Err(AppError::Validation(
            "Password must not exceed 72 characters".to_string(),
        ));
    }

    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password
        .chars()
        .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?/~`".contains(c));

    if !has_uppercase || !has_lowercase || !has_digit || !has_special {
        return Err(AppError::Validation(
            "Password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character".to_string(),
        ));
    }

    Ok(())
}

/// Generates a signed JWT access token.
///
/// # Arguments
///
/// * `user_id` - User UUID
/// * `role` - User role
/// * `email` - User email
/// * `secret` - JWT signing secret
/// * `hours` - Expiration duration in hours
///
/// # Errors
///
/// Returns [`AppError::Internal`] if JWT encoding fails.
pub fn generate_access_token(
    user_id: Uuid,
    role: UserRole,
    email: &str,
    secret: &str,
    hours: i64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let expiration = now + Duration::hours(hours);

    let claims = Claims {
        sub: user_id,
        role,
        email: email.to_string(),
        exp: expiration.timestamp().max(0) as usize,
        iat: now.timestamp().max(0) as usize,
        iss: "telemetry-api".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT generation error: {}", e)))
}

/// Verifies and decodes a JWT access token.
///
/// # Arguments
///
/// * `token` - Bearer JWT token string
/// * `secret` - JWT secret key
///
/// # Errors
///
/// Returns [`AppError::Unauthorized`] if the token is expired or has an invalid signature.
pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.set_issuer(&["telemetry-api"]);

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            AppError::Unauthorized("Token has expired".to_string())
        }
        _ => AppError::Unauthorized("Invalid token signature".to_string()),
    })
}

/// Generates a cryptographically secure random 256-bit token encoded as a hex string.
pub fn generate_secure_random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Computes the SHA-256 hash of a refresh token for secure database storage.
///
/// # Arguments
///
/// * `token` - Plaintext refresh token string
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_password_hashing_and_verification() {
        let password = "SecurePassword123!";
        let hash = hash_password(password).expect("Hashing should succeed");

        assert_ne!(password, hash);
        assert!(
            verify_password(password, &hash).expect("Verification"),
            "Valid password must verify"
        );
        assert!(
            !verify_password("WrongPassword123!", &hash).expect("Verification"),
            "Wrong password must not verify"
        );
    }

    #[test]
    fn test_auth_jwt_token_generation_and_expiry() {
        let user_id = Uuid::new_v4();
        let secret = "super-secret-key-at-least-32-characters-long!";
        let email = "admin@example.com";

        // Generate expired token (-1 hours)
        let expired_token = generate_access_token(user_id, UserRole::Admin, email, secret, -1)
            .expect("Generation should succeed");

        let result = verify_jwt(&expired_token, secret);
        assert!(
            matches!(result, Err(AppError::Unauthorized(ref msg)) if msg.contains("expired")),
            "Expired token must return Unauthorized expired error"
        );

        // Generate valid token
        let valid_token = generate_access_token(user_id, UserRole::Admin, email, secret, 24)
            .expect("Generation should succeed");
        let claims = verify_jwt(&valid_token, secret).expect("Valid token must verify");
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.role, UserRole::Admin);
        assert_eq!(claims.email, email);
    }

    #[test]
    fn test_auth_password_policy_validation() {
        // Short password
        let err = validate_password_complexity("Ab1!").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Missing special char
        let err = validate_password_complexity("Abcdefgh123").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Missing uppercase
        let err = validate_password_complexity("abcdefgh123!").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Missing lowercase
        let err = validate_password_complexity("ABCDEFGH123!").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Missing digit
        let err = validate_password_complexity("Abcdefghijk!").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));

        // Valid password
        let ok = validate_password_complexity("ValidPass123!");
        assert!(ok.is_ok());
    }

    #[test]
    fn test_auth_token_hashing_and_random() {
        let token = generate_secure_random_token();
        assert_eq!(token.len(), 64); // 32 bytes in hex = 64 characters

        let hash1 = hash_token(&token);
        let hash2 = hash_token(&token);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }
}
