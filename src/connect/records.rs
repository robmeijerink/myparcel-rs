use std::future::Future;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::connect::error::StorageError;

const INSTALLATION_VERSION: u8 = 2;
const TOKEN_VERSION: u8 = 1;
const NONCES_VERSION: u8 = 1;

fn read<T: for<'de> Deserialize<'de>>(value: &Value, current: u8) -> Option<T> {
    let version = value.get("version")?.as_u64()?;
    if version == 0 || version > u64::from(current) {
        return None;
    }
    serde_json::from_value(value.clone()).ok()
}

/// What is stored about a shop for as long as it exists: its key and its connection.
///
/// The `version` field lets a newer record format be told apart from an older one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    version: u8,
    /// The private key, encrypted.
    pub encrypted_private_key: String,
    /// The id of the connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// The method the proofs of the token endpoints use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_htm: Option<String>,
    /// The url the proofs of the token endpoints use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge_htu: Option<String>,
    /// When the refresh token expires, in seconds since the epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token_expires_at: Option<u64>,
}

impl Installation {
    pub(crate) fn new(encrypted_private_key: String) -> Self {
        Self {
            version: INSTALLATION_VERSION,
            encrypted_private_key,
            connection_id: None,
            challenge_htm: None,
            challenge_htu: None,
            refresh_token_expires_at: None,
        }
    }

    /// The record as JSON.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("a record always serializes")
    }

    /// Reads a record. Gives `None` for a missing or newer version and for unusable fields.
    pub fn from_value(value: &Value) -> Option<Self> {
        let record: Self = read(value, INSTALLATION_VERSION)?;
        (!record.encrypted_private_key.is_empty()).then_some(record)
    }
}

/// The access token of a shop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    version: u8,
    /// The access token, encrypted.
    pub encrypted_access_token: String,
    /// When the access token expires, in seconds since the epoch.
    pub expires_at: u64,
    /// The scope of the token.
    pub scope: String,
}

impl Token {
    pub(crate) fn new(encrypted_access_token: String, expires_at: u64, scope: String) -> Self {
        Self {
            version: TOKEN_VERSION,
            encrypted_access_token,
            expires_at,
            scope,
        }
    }

    /// The record as JSON.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("a record always serializes")
    }

    /// Reads a record. Gives `None` for a missing or newer version and for unusable fields.
    pub fn from_value(value: &Value) -> Option<Self> {
        let record: Self = read(value, TOKEN_VERSION)?;
        (!record.encrypted_access_token.is_empty()).then_some(record)
    }
}

/// Short-lived values of the protocol: losing them costs one extra round trip.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nonces {
    #[serde(default)]
    version: u8,
    /// The nonce of the pending start, until it is used or expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_nonce: Option<String>,
    /// When the start nonce expires, in seconds since the epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_nonce_expires_at: Option<u64>,
    /// The nonce MyParcel asked the proofs to carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dpop_nonce: Option<String>,
}

impl Nonces {
    pub(crate) fn current() -> Self {
        Self {
            version: NONCES_VERSION,
            ..Self::default()
        }
    }

    /// The record as JSON.
    pub fn to_value(&self) -> Value {
        let mut record = self.clone();
        record.version = NONCES_VERSION;
        serde_json::to_value(record).expect("a record always serializes")
    }

    /// Reads a record. Gives `None` for a missing or newer version.
    pub fn from_value(value: &Value) -> Option<Self> {
        read(value, NONCES_VERSION)
    }
}

/// Where the state of one shop's connection is kept, for the asynchronous service.
///
/// One storage holds one shop. Records contain encrypted secrets but also plain data; keep them
/// out of web-served paths and logs and treat backups as credentials.
pub trait ConnectStorage: Send + Sync {
    /// Loads the installation.
    fn load_installation(
        &self,
    ) -> impl Future<Output = Result<Option<Installation>, StorageError>> + Send;
    /// Stores the installation.
    fn save_installation(
        &self,
        installation: &Installation,
    ) -> impl Future<Output = Result<(), StorageError>> + Send;
    /// Loads the token.
    fn load_token(&self) -> impl Future<Output = Result<Option<Token>, StorageError>> + Send;
    /// Stores the token, or removes it with `None`.
    fn save_token(
        &self,
        token: Option<&Token>,
    ) -> impl Future<Output = Result<(), StorageError>> + Send;
    /// Loads the nonces.
    fn load_nonces(&self) -> impl Future<Output = Result<Option<Nonces>, StorageError>> + Send;
    /// Stores the nonces, or removes them with `None`.
    fn save_nonces(
        &self,
        nonces: Option<&Nonces>,
    ) -> impl Future<Output = Result<(), StorageError>> + Send;
    /// Removes everything.
    fn clear(&self) -> impl Future<Output = Result<(), StorageError>> + Send;
}

/// Where the state of one shop's connection is kept, for the blocking service.
pub trait BlockingConnectStorage: Send + Sync {
    /// Loads the installation.
    fn load_installation(&self) -> Result<Option<Installation>, StorageError>;
    /// Stores the installation.
    fn save_installation(&self, installation: &Installation) -> Result<(), StorageError>;
    /// Loads the token.
    fn load_token(&self) -> Result<Option<Token>, StorageError>;
    /// Stores the token, or removes it with `None`.
    fn save_token(&self, token: Option<&Token>) -> Result<(), StorageError>;
    /// Loads the nonces.
    fn load_nonces(&self) -> Result<Option<Nonces>, StorageError>;
    /// Stores the nonces, or removes them with `None`.
    fn save_nonces(&self, nonces: Option<&Nonces>) -> Result<(), StorageError>;
    /// Removes everything.
    fn clear(&self) -> Result<(), StorageError>;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn records_have_the_documented_field_names() {
        let mut installation = Installation::new("sealed".into());
        installation.connection_id = Some("c1".into());
        installation.refresh_token_expires_at = Some(99);
        assert_eq!(
            installation.to_value(),
            json!({"version": 2, "encryptedPrivateKey": "sealed", "connectionId": "c1", "refreshTokenExpiresAt": 99})
        );
        assert_eq!(
            Token::new("t".into(), 5, "integration".into()).to_value(),
            json!({"version": 1, "encryptedAccessToken": "t", "expiresAt": 5, "scope": "integration"})
        );
    }

    #[test]
    fn newer_missing_or_unusable_records_are_ignored() {
        assert!(
            Installation::from_value(&json!({"version": 3, "encryptedPrivateKey": "x"})).is_none()
        );
        assert!(Installation::from_value(&json!({"encryptedPrivateKey": "x"})).is_none());
        assert!(
            Installation::from_value(&json!({"version": 2, "encryptedPrivateKey": ""})).is_none()
        );
        assert!(
            Token::from_value(&json!({"version": 1, "encryptedAccessToken": "t", "scope": "s"}))
                .is_none()
        );
        assert!(
            Installation::from_value(&json!({"version": 1, "encryptedPrivateKey": "x"})).is_some()
        );
    }

    #[test]
    fn nonces_round_trip() {
        let mut nonces = Nonces::current();
        nonces.dpop_nonce = Some("n".into());
        assert_eq!(Nonces::from_value(&nonces.to_value()), Some(nonces));
    }
}
