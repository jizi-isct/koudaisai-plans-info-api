use jwt_simple::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::{Error, Fetch, Headers, Method, Request, RequestInit};

/// JWKS (JSON Web Key Set) structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// JWK (JSON Web Key) structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    pub kty: String,          // Key Type
    pub kid: Option<String>,  // Key ID
    pub use_: Option<String>, // Public Key Use
    pub alg: Option<String>,  // Algorithm
    pub n: Option<String>,    // Modulus (for RSA)
    pub e: Option<String>,    // Exponent (for RSA)
    pub x: Option<String>,    // X coordinate (for EC)
    pub y: Option<String>,    // Y coordinate (for EC)
    pub crv: Option<String>,  // Curve (for EC)
    #[serde(flatten)]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

/// JWT Header structure
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub typ: Option<String>,
    pub kid: Option<String>,
}

/// Error types for JWKS operations
#[derive(Debug)]
pub enum JwksError {
    NetworkError(String),
    ParseError(String),
    KeyNotFound,
    InvalidToken,
    VerificationError(String),
}

impl std::fmt::Display for JwksError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwksError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            JwksError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            JwksError::KeyNotFound => write!(f, "Key not found in JWKS"),
            JwksError::InvalidToken => write!(f, "Invalid JWT token format"),
            JwksError::VerificationError(msg) => write!(f, "Verification error: {}", msg),
        }
    }
}

impl std::error::Error for JwksError {}

impl Into<Error> for JwksError {
    fn into(self) -> Error {
        Error::Internal(format!("{}", self).into())
    }
}

/// Fetch JWKS from a remote URL (without caching)
async fn fetch_jwks_remote(jwks_url: &str) -> Result<Jwks, JwksError> {
    let request = {
        let mut headers = Headers::new();
        headers
            .set("Accept", "application/json")
            .map_err(|e| JwksError::NetworkError(format!("Failed to set headers: {:?}", e)))?;

        Request::new_with_init(
            jwks_url,
            RequestInit::new()
                .with_method(Method::Get)
                .with_headers(headers),
        )
        .map_err(|e| JwksError::NetworkError(format!("Failed to create request: {:?}", e)))?
    };

    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|e| JwksError::NetworkError(format!("Request failed: {:?}", e)))?;

    let status = response.status_code();
    if status < 200 || status >= 300 {
        return Err(JwksError::NetworkError(format!(
            "HTTP {} from JWKS endpoint",
            status
        )));
    }

    let text = response
        .text()
        .await
        .map_err(|e| JwksError::ParseError(format!("Failed to read response: {:?}", e)))?;

    serde_json::from_str::<Jwks>(&text)
        .map_err(|e| JwksError::ParseError(format!("Failed to parse JWKS: {:?}", e)))
}

/// Fetch JWKS without caching
pub async fn fetch_jwks(jwks_url: &str) -> Result<Jwks, JwksError> {
    fetch_jwks_remote(jwks_url).await
}

/// Find a JWK by key ID
pub fn find_jwk_by_kid<'a>(jwks: &'a Jwks, kid: &str) -> Option<&'a Jwk> {
    jwks.keys
        .iter()
        .find(|key| key.kid.as_ref().map(|k| k == kid).unwrap_or(false))
}

/// Parse JWT header to extract algorithm and key ID
pub fn parse_jwt_header(token: &str) -> Result<JwtHeader, JwksError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(JwksError::InvalidToken);
    }

    let header_b64 = parts[0];
    let header_bytes = base64_decode(header_b64).map_err(|_| JwksError::InvalidToken)?;

    let header_str = std::str::from_utf8(&header_bytes).map_err(|_| JwksError::InvalidToken)?;

    serde_json::from_str::<JwtHeader>(header_str)
        .map_err(|e| JwksError::ParseError(format!("Failed to parse JWT header: {:?}", e)))
}

/// Simple base64 URL decode (without padding handling for simplicity)
pub fn base64_decode(input: &str) -> Result<Vec<u8>, JwksError> {
    // Add padding if needed
    let mut padded = input.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }

    // Replace URL-safe characters
    let standard = padded.replace('-', "+").replace('_', "/");

    // For simplicity, we'll use a basic implementation
    // In a real implementation, you'd use a proper base64 library
    base64_decode_simple(&standard)
}

fn base64_decode_simple(input: &str) -> Result<Vec<u8>, JwksError> {
    // This is a simplified base64 decoder for demo purposes
    // In production, use a proper base64 library like `base64` crate
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for ch in input.chars() {
        if ch == '=' {
            break;
        }

        let val =
            BASE64_CHARS
                .iter()
                .position(|&x| x as char == ch)
                .ok_or(JwksError::ParseError(
                    "Invalid base64 character".to_string(),
                ))?;

        buffer = (buffer << 6) | (val as u32);
        bits += 6;

        if bits >= 8 {
            result.push((buffer >> (bits - 8)) as u8);
            buffer &= (1 << (bits - 8)) - 1;
            bits -= 8;
        }
    }

    Ok(result)
}
