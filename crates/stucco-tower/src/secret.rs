//! Shared signing primitives for cookies. Payload domains prevent cross-protocol reuse.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::fmt;

/// A 256-bit application signing key. Keep it stable across restarts and replicas.
/// Debug output never includes key material.
#[derive(Clone)]
pub struct SecretKey([u8; 32]);

impl SecretKey {
    /// Uses 32 cryptographically random bytes supplied by the application.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Generates a key with the operating system's random source.
    pub fn generate() -> Result<Self, SecretKeyError> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| SecretKeyError::Random)?;
        Ok(Self(bytes))
    }

    /// Parses exactly 64 hexadecimal digits without exposing invalid input in errors.
    pub fn from_hex(value: &str) -> Result<Self, SecretKeyError> {
        if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(SecretKeyError::Invalid);
        }
        let mut bytes = [0; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)
                .map_err(|_| SecretKeyError::Invalid)?;
        }
        Ok(Self(bytes))
    }

    /// Reads `STUCCO_SECRET_KEY` as 64 hexadecimal digits.
    /// A missing key generates an ephemeral key with a tracing warning in
    /// debug builds; in release builds it is a startup error. Invalid values
    /// are always errors. Call once at startup and share the resulting key.
    pub fn from_env() -> Result<Self, SecretKeyError> {
        match std::env::var("STUCCO_SECRET_KEY") {
            Ok(value) => Self::from_hex(&value),
            Err(std::env::VarError::NotUnicode(_)) => Err(SecretKeyError::Invalid),
            Err(std::env::VarError::NotPresent) => Self::missing(),
        }
    }

    fn missing() -> Result<Self, SecretKeyError> {
        if cfg!(debug_assertions) {
            tracing::warn!("STUCCO_SECRET_KEY is missing; using an ephemeral development key");
            Self::generate()
        } else {
            Err(SecretKeyError::Missing)
        }
    }

    pub(crate) fn sign(&self, domain: &[u8], payload: &[u8]) -> String {
        URL_SAFE_NO_PAD.encode(self.mac(domain, payload).finalize().into_bytes())
    }

    pub(crate) fn verify(&self, domain: &[u8], payload: &[u8], tag: &str) -> bool {
        // Bound allocation and compare MAC bytes in constant time.
        if tag.len() != 43 {
            return false;
        }
        let Ok(tag) = URL_SAFE_NO_PAD.decode(tag) else {
            return false;
        };
        self.mac(domain, payload).verify_slice(&tag).is_ok()
    }

    fn mac(&self, domain: &[u8], payload: &[u8]) -> Hmac<Sha256> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.0).expect("HMAC accepts a 32-byte key");
        mac.update(domain);
        mac.update(payload);
        mac
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey([redacted])")
    }
}

/// A signing key could not be configured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKeyError {
    /// Release builds require an explicitly configured key.
    Missing,
    /// The configured key is not 64 hexadecimal digits.
    Invalid,
    /// The operating system's random source failed.
    Random,
}
impl fmt::Display for SecretKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Missing => "STUCCO_SECRET_KEY is required in release builds",
            Self::Invalid => "STUCCO_SECRET_KEY must contain 64 hexadecimal digits",
            Self::Random => "could not generate a signing key",
        })
    }
}
impl std::error::Error for SecretKeyError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_are_validated_redacted_and_domain_separated() {
        let key = SecretKey::from_hex(&"ab".repeat(32)).unwrap();
        assert_eq!(format!("{key:?}"), "SecretKey([redacted])");
        for invalid in ["", "ff", &"gg".repeat(32), &"é".repeat(32)] {
            assert_eq!(
                SecretKey::from_hex(invalid).unwrap_err(),
                SecretKeyError::Invalid
            );
        }
        let tag = key.sign(b"csrf", b"payload");
        assert!(key.verify(b"csrf", b"payload", &tag));
        assert!(!key.verify(b"flash", b"payload", &tag));
        assert!(!key.verify(b"csrf", b"changed", &tag));
        assert!(!SecretKey::new([0; 32]).verify(b"csrf", b"payload", &tag));
    }
    #[test]
    fn missing_key_policy_matches_the_build() {
        let result = SecretKey::missing();
        if cfg!(debug_assertions) {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.unwrap_err(), SecretKeyError::Missing);
        }
    }
}
