use serde_json::Value;

use crate::error::ValidationError;
use crate::reported::Reported;

/// The error type of what a storage implementation can fail with.
pub type StorageError = Box<dyn std::error::Error + Send + Sync>;

string_enum! {
    /// An error code MyParcel Connect answers with.
    pub enum ErrorCode {
        /// A DPoP nonce is required.
        UseDpopNonce = "use_dpop_nonce",
        /// The refresh token or code is no longer valid.
        InvalidGrant = "invalid_grant",
        /// The DPoP proof was refused.
        InvalidDpopProof = "invalid_dpop_proof",
        /// The shop could not be linked to a sales channel.
        InvalidSalesChannel = "invalid_sales_channel",
        /// The code was not recognised.
        InvalidCode = "invalid_code",
        /// MyParcel failed.
        ServerError = "server_error",
    }
}

string_enum! {
    /// An error MyParcel Connect reports in the callback.
    pub enum CallbackError {
        /// The login was cancelled, or the account has no order management.
        AccessDenied = "access_denied",
        /// The connect session expired.
        SessionExpired = "session_expired",
        /// The shop could not be linked to a sales channel.
        InvalidSalesChannel = "invalid_sales_channel",
        /// MyParcel failed.
        ServerError = "server_error",
    }
}

/// Errors of MyParcel Connect.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConnectError {
    /// The login was cancelled or denied.
    ///
    /// MyParcel also reports this for accounts without order management, so check the account's
    /// features before telling a merchant they cancelled.
    #[error("the connection was denied")]
    AccessDenied,
    /// The connect session expired; start again.
    #[error("the connect session expired, start again")]
    SessionExpired,
    /// MyParcel could not link the shop to a sales channel.
    #[error("MyParcel could not link this shop to a sales channel")]
    InvalidSalesChannel,
    /// MyParcel failed to complete the connection.
    #[error("MyParcel could not complete the connection{}", .0.as_deref().map(|v| format!(" (reported as \"{v}\")")).unwrap_or_default())]
    ServerError(Option<String>),
    /// The callback was refused.
    #[error("the connect callback was refused: {0}")]
    InvalidCallback(String),
    /// The connection was revoked; the shop has to connect again.
    #[error("the connection was revoked, the shop has to connect again")]
    ConnectionRevoked,
    /// The shop is not connected.
    #[error("this shop is not connected to MyParcel")]
    NotConnected,
    /// The access token is bound to another key than the one of this shop.
    #[error("the access token is not bound to this shop's key")]
    KeyMismatch,
    /// MyParcel keeps asking for a new nonce.
    #[error("MyParcel keeps asking for a new nonce")]
    NonceRequired,
    /// Stored state could not be encrypted.
    #[error("could not encrypt the connect state")]
    Encryption,
    /// Stored state could not be decrypted. Check the encryption key.
    #[error("could not decrypt the stored connect state, check the encryption key")]
    Decryption,
    /// MyParcel answered with an error.
    #[error("MyParcel answered {status}")]
    Http {
        /// The HTTP status.
        status: u16,
        /// The error code, when MyParcel sent one.
        code: Option<Reported<ErrorCode>>,
        /// The response body, when it was JSON.
        body: Option<Value>,
    },
    /// MyParcel could not be reached.
    #[error("could not reach MyParcel: {0}")]
    Network(#[from] reqwest::Error),
    /// The storage failed.
    #[error("the connect storage failed: {0}")]
    Storage(StorageError),
    /// A value was rejected before anything was sent.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    /// A response did not have the expected shape.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(&'static str),
}

impl ConnectError {
    pub(crate) fn from_callback(error: &str) -> Self {
        match CallbackError::from_name(error) {
            Some(CallbackError::AccessDenied) => Self::AccessDenied,
            Some(CallbackError::SessionExpired) => Self::SessionExpired,
            Some(CallbackError::InvalidSalesChannel) => Self::InvalidSalesChannel,
            Some(CallbackError::ServerError) => Self::ServerError(None),
            None => Self::ServerError(Some(error.to_owned())),
        }
    }
}
