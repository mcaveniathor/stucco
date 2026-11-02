//! Signed single-submit tokens, so a double tap on a form takes effect once.
use crate::{SecretKey, SecretKeyError};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DOMAIN: &[u8] = b"stucco:submit:v1\0";
const EXPIRES: usize = 8;
const NONCE: usize = 16;
// 24 payload bytes, a dot, and a 32-byte MAC, all but the dot in base64url.
const TOKEN_LEN: usize = 32 + 1 + 43;

/// Issues and verifies an idempotency token for each rendered form.
///
/// Issue a token per rendering with [`issue`](Self::issue) and put it in the
/// form with `Form::single_submit`. When the form comes back,
/// [`verify`](Self::verify) proves this application issued the token for this
/// `scope` and that it has not expired, and returns the [`SubmitKey`] that
/// identifies that rendering.
///
/// Verifying does not spend a token: this type holds no state. Record
/// [`SubmitKey::id`] in the same transaction as the form's effect, and treat a
/// repeated id as a repeat of the first submission. Keep each record until
/// [`SubmitKey::expires_at`]; an expired token no longer verifies, so older
/// records can go. `stucco_redb::SubmitLedger` does this for redb.
///
/// `scope` binds a token to what the form is for and who it was shown to, such
/// as `"complete:7:member-12"`, so it cannot be moved to another form or
/// account. Pair this with [`CsrfLayer`](crate::CsrfLayer): CSRF protection
/// proves where a request came from, and this token proves it is new.
///
/// ```
/// use stucco_tower::{SecretKey, SingleSubmit};
/// let forms = SingleSubmit::new(SecretKey::new([7; 32]));
/// let token = forms.issue("orders:new").unwrap();
/// let key = forms.verify("orders:new", Some(token.as_str())).unwrap();
/// assert_eq!(key.id().len(), 22);
/// assert!(forms.verify("orders:edit", Some(token.as_str())).is_err());
/// ```
#[derive(Clone, Debug)]
pub struct SingleSubmit {
    key: SecretKey,
    lifetime: u64,
}

impl SingleSubmit {
    /// Uses a stable application key; tokens last one hour by default. The key
    /// may be shared with the CSRF and flash layers: a separate signing domain
    /// keeps their signatures apart.
    pub fn new(key: SecretKey) -> Self {
        Self {
            key,
            lifetime: 3600,
        }
    }

    /// How long a form stays submittable, rounded down to whole seconds.
    pub fn lifetime(mut self, lifetime: Duration) -> Self {
        self.lifetime = lifetime.as_secs();
        self
    }

    /// A fresh token for one rendering of a form.
    pub fn issue(&self, scope: &str) -> Result<SubmitToken, SecretKeyError> {
        let mut nonce = [0; NONCE];
        getrandom::fill(&mut nonce).map_err(|_| SecretKeyError::Random)?;
        let expires = unix_now()?.saturating_add(self.lifetime);
        Ok(self.sign(scope, expires, nonce))
    }

    /// Checks a submitted token, as given by `FormState::submit_token`.
    pub fn verify(&self, scope: &str, token: Option<&str>) -> Result<SubmitKey, SubmitRejection> {
        let now = unix_now().map_err(|_| SubmitRejection::Invalid)?;
        self.verify_at(scope, token, now)
    }

    fn sign(&self, scope: &str, expires: u64, nonce: [u8; NONCE]) -> SubmitToken {
        let mut payload = [0; EXPIRES + NONCE];
        payload[..EXPIRES].copy_from_slice(&expires.to_be_bytes());
        payload[EXPIRES..].copy_from_slice(&nonce);
        let tag = self.key.sign(DOMAIN, &signed(&payload, scope));
        SubmitToken(format!("{}.{tag}", URL_SAFE_NO_PAD.encode(payload)))
    }

    fn verify_at(
        &self,
        scope: &str,
        token: Option<&str>,
        now: u64,
    ) -> Result<SubmitKey, SubmitRejection> {
        let token = token.ok_or(SubmitRejection::Missing)?;
        if token.len() != TOKEN_LEN {
            return Err(SubmitRejection::Invalid);
        }
        let (payload, tag) = token.split_once('.').ok_or(SubmitRejection::Invalid)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| SubmitRejection::Invalid)?;
        if payload.len() != EXPIRES + NONCE
            || !self.key.verify(DOMAIN, &signed(&payload, scope), tag)
        {
            return Err(SubmitRejection::Invalid);
        }
        let (expires, nonce) = payload.split_at(EXPIRES);
        let expires = u64::from_be_bytes(expires.try_into().map_err(|_| SubmitRejection::Invalid)?);
        if expires <= now {
            return Err(SubmitRejection::Expired);
        }
        Ok(SubmitKey {
            id: URL_SAFE_NO_PAD.encode(nonce),
            expires_at: expires,
        })
    }
}

// The payload is fixed-width, so the scope that follows it cannot blur into it.
fn signed(payload: &[u8], scope: &str) -> Vec<u8> {
    [payload, scope.as_bytes()].concat()
}

fn unix_now() -> Result<u64, SecretKeyError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| SecretKeyError::Random)
}

/// A token for one rendering of a form, from [`SingleSubmit::issue`].
/// Pass `as_str()` to `Form::single_submit`.
#[derive(Clone)]
pub struct SubmitToken(String);

impl SubmitToken {
    /// The value for a hidden `_submit` field.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SubmitToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SubmitToken([redacted])")
    }
}

/// The identity of one rendering of a form, from [`SingleSubmit::verify`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmitKey {
    id: String,
    expires_at: u64,
}

impl SubmitKey {
    /// A random id, unique to the rendering. Record it with the form's effect.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Seconds since the Unix epoch at which the token stops verifying.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}

/// Why [`SingleSubmit::verify`] refused a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubmitRejection {
    /// The form had no token, or more than one.
    Missing,
    /// The token was altered, or issued by another key or for another scope.
    Invalid,
    /// The form was open too long. Show it again with a new token.
    Expired,
}

impl fmt::Display for SubmitRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Missing => "the form has no single-submit token",
            Self::Invalid => "the form's single-submit token is not valid here",
            Self::Expired => "the form's single-submit token has expired",
        })
    }
}

impl std::error::Error for SubmitRejection {}

#[cfg(test)]
mod tests {
    use super::*;

    fn forms() -> SingleSubmit {
        SingleSubmit::new(SecretKey::new([7; 32]))
    }

    #[test]
    fn tokens_verify_until_they_expire() {
        let token = forms().sign("scope", 1000, [1; NONCE]);
        let check = |now| forms().verify_at("scope", Some(token.as_str()), now);
        let key = check(999).unwrap();
        assert_eq!(key.expires_at(), 1000);
        assert_eq!(key.id(), URL_SAFE_NO_PAD.encode([1; NONCE]));
        assert_eq!(check(1000), Err(SubmitRejection::Expired));
        assert_eq!(check(1001), Err(SubmitRejection::Expired));
    }

    #[test]
    fn tokens_are_bound_to_their_key_and_scope() {
        let token = forms().issue("a:1").unwrap();
        let other = SingleSubmit::new(SecretKey::new([8; 32]));
        for result in [
            forms().verify("a:2", Some(token.as_str())),
            other.verify("a:1", Some(token.as_str())),
        ] {
            assert_eq!(result, Err(SubmitRejection::Invalid));
        }
    }

    #[test]
    fn altered_or_malformed_tokens_are_invalid() {
        let token = forms().issue("scope").unwrap();
        let mut altered = token.as_str().as_bytes().to_vec();
        altered[9] = if altered[9] == b'A' { b'B' } else { b'A' };
        let altered = String::from_utf8(altered).unwrap();
        let long = "a".repeat(TOKEN_LEN + 1);
        let dotless = "a".repeat(TOKEN_LEN);
        for bad in [altered.as_str(), "", "a.b", &long, &dotless] {
            assert_eq!(
                forms().verify("scope", Some(bad)),
                Err(SubmitRejection::Invalid)
            );
        }
        assert_eq!(forms().verify("scope", None), Err(SubmitRejection::Missing));
    }

    #[test]
    fn each_rendering_gets_its_own_id_and_a_configurable_lifetime() {
        let forms = forms().lifetime(Duration::from_secs(60));
        let read = |token: SubmitToken| forms.verify("scope", Some(token.as_str())).unwrap();
        let (first, second) = (
            read(forms.issue("scope").unwrap()),
            read(forms.issue("scope").unwrap()),
        );
        assert_ne!(first.id(), second.id());
        let now = unix_now().unwrap();
        assert!((now..=now + 60).contains(&first.expires_at()));
        assert_eq!(
            format!("{:?}", forms.issue("x").unwrap()),
            "SubmitToken([redacted])"
        );
    }
}
