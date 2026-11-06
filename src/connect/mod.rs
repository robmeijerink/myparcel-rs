//! MyParcel Connect: OAuth with proof of possession for published integrations.
//!
//! A merchant logs in at MyParcel and is redirected back to the shop. The shop proves that it
//! holds a key (DPoP, RFC 9449, ES256) on every request, so a stolen token is useless without
//! the key. Connect currently covers pushing orders; everything else needs an API key.

mod config;
mod crypto;
mod ecommerce;
mod error;
mod protocol;
mod records;
mod service;

pub use config::{CALLBACK_PATH, ConnectConfig, ConnectPlatform, ConnectScope, EncryptionKey};
pub use ecommerce::*;
pub use error::{CallbackError, ConnectError, ErrorCode, StorageError};
pub use protocol::{AccessToken, CallbackQuery, PluginVersion, ShopName, ShopUrl};
pub use records::{BlockingConnectStorage, ConnectStorage, Installation, Nonces, Token};
#[cfg(feature = "blocking")]
pub use service::BlockingConnectService;
pub use service::ConnectService;
