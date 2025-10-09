use crate::jwks::{fetch_jwks, find_jwk_by_kid, parse_jwt_header, Jwks, JwksError};
use jwt_simple::algorithms::{RS256PublicKey, RS384PublicKey, RS512PublicKey, RSAPublicKeyLike};
use jwt_simple::claims::NoCustomClaims;
use thiserror::Error;
use worker::Headers;

#[derive(Clone)]
pub struct JwtVerifier {
    jwks: Jwks,
}

impl JwtVerifier {
    pub async fn new(jwks_url: &str) -> Result<JwtVerifier, JwksError> {
        fetch_jwks(jwks_url).await.map(|jwks| Self { jwks })
    }

    pub fn verify_token(&self, token: &str) -> Result<bool, JwksError> {
        // Parse JWT header
        let header = parse_jwt_header(token)?;

        // Get key ID from header
        let kid = header.kid.ok_or(JwksError::KeyNotFound)?;

        // Find the appropriate key
        let jwk = find_jwk_by_kid(&self.jwks, &kid).ok_or(JwksError::KeyNotFound)?;

        // Verify the token based on the key type and algorithm
        match (jwk.kty.as_str(), header.alg.as_str()) {
            ("RSA", "RS256") => {
                // Create RSA public key from JWK
                let n = jwk.n.as_ref().ok_or(JwksError::KeyNotFound)?;
                let e = jwk.e.as_ref().ok_or(JwksError::KeyNotFound)?;

                // Decode base64url encoded modulus and exponent
                let n_bytes = crate::jwks::base64_decode(n)?;
                let e_bytes = crate::jwks::base64_decode(e)?;

                // Create RSA public key
                let public_key = RS256PublicKey::from_components(&n_bytes, &e_bytes)
                    .map_err(|e| JwksError::VerificationError(format!("Failed to create RSA key: {:?}", e)))?;

                // Verify the token
                match public_key.verify_token::<NoCustomClaims>(token, None) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(JwksError::VerificationError(format!("JWT verification failed: {:?}", e))),
                }
            }
            ("RSA", "RS384") => {
                // Create RSA public key from JWK for RS384
                let n = jwk.n.as_ref().ok_or(JwksError::KeyNotFound)?;
                let e = jwk.e.as_ref().ok_or(JwksError::KeyNotFound)?;

                let n_bytes = crate::jwks::base64_decode(n)?;
                let e_bytes = crate::jwks::base64_decode(e)?;

                let public_key = RS384PublicKey::from_components(&n_bytes, &e_bytes)
                    .map_err(|e| JwksError::VerificationError(format!("Failed to create RSA key: {:?}", e)))?;

                match public_key.verify_token::<NoCustomClaims>(token, None) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(JwksError::VerificationError(format!("JWT verification failed: {:?}", e))),
                }
            }
            ("RSA", "RS512") => {
                // Create RSA public key from JWK for RS512
                let n = jwk.n.as_ref().ok_or(JwksError::KeyNotFound)?;
                let e = jwk.e.as_ref().ok_or(JwksError::KeyNotFound)?;

                let n_bytes = crate::jwks::base64_decode(n)?;
                let e_bytes = crate::jwks::base64_decode(e)?;

                let public_key = RS512PublicKey::from_components(&n_bytes, &e_bytes)
                    .map_err(|e| JwksError::VerificationError(format!("Failed to create RSA key: {:?}", e)))?;

                match public_key.verify_token::<NoCustomClaims>(token, None) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(JwksError::VerificationError(format!("JWT verification failed: {:?}", e))),
                }
            }
            (kty, alg) => {
                Err(JwksError::VerificationError(format!(
                    "Unsupported key type '{}' or algorithm '{}'. Currently supported: RSA with RS256/RS384/RS512",
                    kty, alg
                )))
            }
        }
    }

    pub fn verify_token_in_headers(
        &self,
        headers: &Headers,
    ) -> Result<bool, VerifyTokenInHeadersError> {
        let Some(authorization) = headers.get("Authorization")? else {
            return Err(VerifyTokenInHeadersError::MissingAuthorizationHeader);
        };

        let Some(token) = authorization.strip_prefix("Bearer ") else {
            return Err(VerifyTokenInHeadersError::InvalidAuthorizationHeader);
        };

        Ok(self.verify_token(token)?)
    }
}

#[derive(Debug, Error)]
pub enum VerifyTokenInHeadersError {
    #[error("Missing Authorization header")]
    MissingAuthorizationHeader,
    #[error("Invalid Authorization header")]
    InvalidAuthorizationHeader,
    #[error("JWT verification failed: {0}")]
    JwksError(#[from] JwksError),
    #[error(transparent)]
    WorkersError(#[from] worker::Error),
}
