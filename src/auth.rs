use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::header::HeaderValue;
use zeroize::Zeroizing;

use crate::error::ValidationError;

const ENV_KEYS: [&str; 3] = ["API_KEY", "API_KEY_NL", "API_KEY_BE"];

/// A MyParcel API key.
///
/// This copy of the key is wiped from memory on drop, never shown by `Debug` and sent as a
/// sensitive header. The copy inside a request header, held by the HTTP client while the request
/// runs, is not wiped.
#[derive(Clone)]
pub struct ApiKey(Zeroizing<String>);

impl ApiKey {
    /// Wraps an API key, rejecting empty keys and keys with whitespace or control characters.
    pub fn new(key: impl Into<String>) -> Result<Self, ValidationError> {
        let key = Zeroizing::new(key.into());
        if key.is_empty() {
            return Err(ValidationError::new("api key", "must not be empty"));
        }
        if key.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(ValidationError::new(
                "api key",
                "must not contain whitespace or control characters",
            ));
        }
        Ok(Self(key))
    }

    /// Reads the key from `API_KEY`, then `API_KEY_NL`, then `API_KEY_BE`.
    ///
    /// Variables that are set but empty are skipped.
    pub fn from_env() -> Result<Self, ValidationError> {
        ENV_KEYS
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty())
            .ok_or_else(|| {
                ValidationError::new(
                    "api key",
                    "none of API_KEY, API_KEY_NL or API_KEY_BE is set",
                )
            })
            .and_then(Self::new)
    }

    pub(crate) fn bearer(&self) -> HeaderValue {
        let encoded = Zeroizing::new(STANDARD.encode(self.0.as_bytes()));
        let mut value = HeaderValue::from_str(&format!("bearer {}", encoded.as_str()))
            .expect("base64 output is a valid header value");
        value.set_sensitive(true);
        value
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey([redacted])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_keys() {
        assert!(ApiKey::new("").is_err());
        assert!(ApiKey::new("a b").is_err());
        assert!(ApiKey::new("a\nb").is_err());
    }

    #[test]
    fn debug_hides_the_key() {
        let key = ApiKey::new("secret").unwrap();
        assert_eq!(format!("{key:?}"), "ApiKey([redacted])");
    }

    #[test]
    fn bearer_is_base64_and_sensitive() {
        let value = ApiKey::new("test-key").unwrap().bearer();
        assert_eq!(value.to_str().unwrap(), "bearer dGVzdC1rZXk=");
        assert!(value.is_sensitive());
    }
}
